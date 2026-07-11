/**
 * Headless UPS (User Prove Session) controller.
 *
 * Lifts the login/register/identity core from the app's unified/session.ts and
 * the session lifecycle (phases, re-entrancy guard, accountsChanged→logout,
 * persisted auth-version) from UnifiedSessionContext.tsx into a framework-
 * agnostic class. The ./react entry (P6) wraps this in a Provider; nothing here
 * imports React.
 *
 * TRUE MODE-A login:
 *   1. Connect wallet, personal_sign (or EIP-712 v2) the stable auth message —
 *      derives PRIVACY material (r0/r1 + Nostr key), NOT a signing key.
 *   2. Register the wallet ACCOUNT's secp256k1 PUBLIC key via ONE more
 *      personal_sign (Mode-A register): the private key NEVER leaves the wallet,
 *      the app holds NO key. pkHash = hash of that public key.
 *   3. Bind pkHash → evmAddress so every later send/claim authorizes through a
 *      per-tx personal_sign popup.
 *   4. Resolve the on-chain userId (bounded checkpoint-aware poll) + derive the
 *      shield address.
 */

import type { PsyWalletRuntime } from '../runtime'
import { EvmSigner } from '../evm/signer'
import { RpcClient } from '../psy/rpc'
import { ModeARegistry } from './mode-a-registry'
import { ModeASubmitter } from './mode-a-submitter'
import {
  buildAuthMessage,
  buildEip712AuthDomain,
  buildEip712AuthValue,
  deriveKeyMaterialFromSignature,
  deriveShieldAddressForUser,
  DEFAULT_AUTH_VERSION,
  EIP712_AUTH_TYPES,
  type AuthVersion,
} from './identity'
import {
  pollUserIdAcrossCheckpoints,
  type UserIdPollDeps,
  type UserIdPollProgress,
} from './userIdPoll'

export type { UserIdPollProgress } from './userIdPoll'
export type { AuthVersion } from './identity'

/** UI-facing login lifecycle phase (mirrors UnifiedSessionContext's Phase). */
export type SessionPhase =
  | 'idle'
  | 'connecting'
  | 'preparing-prover'
  | 'finishing-registration'
  | 'authed'
  | 'error'

export interface UnifiedSession {
  evmAddress: string
  /** On-chain Psy user id (decimal string), bound to the secp256k1 key. */
  userId: string
  /** Public-key HASH the chain indexes the user under. Under Mode-A this is the
   *  hash of the wallet ACCOUNT's secp256k1 PUBLIC key; every send/claim
   *  authorizes against it via a per-tx personal_sign. */
  pkHash: string
  realmId: number
  /** Canonical 4×u64 shield (shielded) address, colon-joined hex. */
  shieldAddress: string | null
  random0: string
  random1: string
  nostrNpub: string
  nostrSecretKeyHex: string
  /** Which auth-signature variant derived this identity (display-only). NEVER
   *  used in logic — switching it would re-derive a different account. */
  authVersion: AuthVersion
}

/** localStorage key for the persisted (pre-login) auth-version choice. */
const AUTH_VERSION_STORAGE_KEY = 'psy.unified.authVersion'

function normalizeUserId(raw: unknown): string | null {
  const s = String(raw ?? '')
    .trim()
    .replace(/^"+|"+$/g, '')
  return /^\d+$/.test(s) ? s : null
}

export interface SessionState {
  session: UnifiedSession | null
  phase: SessionPhase
  error: string | null
  /** Pre-login auth-version choice (v1 default), honored by the next login. */
  authVersion: AuthVersion
}

export class SessionController {
  private readonly signer: EvmSigner
  private readonly rpc: RpcClient
  readonly registry: ModeARegistry
  readonly submitter: ModeASubmitter

  private state: SessionState
  /** Re-entrancy guard: a second login() while one is in flight is a no-op, so a
   *  double-click never opens two wallet prompts or races two prover spawns. */
  private loginInFlight = false
  private readonly listeners = new Set<(state: SessionState) => void>()
  private unwatchAccounts: (() => void) | null = null

  constructor(private readonly runtime: PsyWalletRuntime) {
    this.signer = new EvmSigner(runtime.wagmiConfig)
    this.rpc = new RpcClient(runtime.network.urls, runtime.network.psy.users_per_realm)
    this.registry = new ModeARegistry()
    this.submitter = new ModeASubmitter(runtime.prover, this.signer)
    this.state = {
      session: null,
      phase: 'idle',
      error: null,
      authVersion: this.loadAuthVersion(),
    }
    // Wallet account switch ⇒ drop the session (identity is account-bound).
    this.unwatchAccounts = this.signer.onAccountsChanged((accounts) => {
      const current = this.state.session
      if (!current) return
      const next = accounts[0]
      if (!next || next.toLowerCase() !== current.evmAddress.toLowerCase()) {
        this.logout()
      }
    })
  }

  // ── state + subscription ────────────────────────────────────────────────────

  getState(): SessionState {
    return this.state
  }

  current(): UnifiedSession | null {
    return this.state.session
  }

  status(): SessionPhase {
    return this.state.phase
  }

  onChange(handler: (state: SessionState) => void): () => void {
    this.listeners.add(handler)
    return () => this.listeners.delete(handler)
  }

  private setState(patch: Partial<SessionState>): void {
    this.state = { ...this.state, ...patch }
    for (const h of this.listeners) {
      try {
        h(this.state)
      } catch {
        // a listener throwing must never break the emit loop
      }
    }
  }

  // ── auth-version choice (persisted) ─────────────────────────────────────────

  private loadAuthVersion(): AuthVersion {
    try {
      return this.runtime.storage.get(AUTH_VERSION_STORAGE_KEY) === 'v2'
        ? 'v2'
        : DEFAULT_AUTH_VERSION
    } catch {
      return DEFAULT_AUTH_VERSION
    }
  }

  /** Change the pre-login auth-version choice. No effect on an active session —
   *  switching after login would re-derive a different account. */
  setAuthVersion(v: AuthVersion): void {
    try {
      this.runtime.storage.set(AUTH_VERSION_STORAGE_KEY, v)
    } catch {
      /* storage unavailable — keep the in-memory choice */
    }
    this.setState({ authVersion: v })
  }

  // ── userId resolution + poll seams ──────────────────────────────────────────

  /** Resolve the on-chain userId for a public-key hash via the coordinator RPC
   *  psy_get_user_ids_for_public_key. Returns null if the key has no user yet. */
  async resolveUserIdForPkHash(pkHash: string): Promise<string | null> {
    const publicKey = pkHash.replace(/^0x/i, '')
    try {
      const result = await this.rpc.coordinatorRpc<unknown[]>(
        'psy_get_user_ids_for_public_key',
        { public_key: publicKey, start_user_id: 0, count: 1 },
      )
      return normalizeUserId(result?.[0] ?? null)
    } catch {
      return null
    }
  }

  private pollDeps(): UserIdPollDeps {
    return {
      resolve: (pkHash) => this.resolveUserIdForPkHash(pkHash),
      getCheckpoint: () => this.rpc.getLatestCheckpointId(),
      sleep: (ms) => new Promise((r) => setTimeout(r, ms)),
      now: () => Date.now(),
    }
  }

  /**
   * Register the wallet ACCOUNT's secp256k1 PUBLIC key as a Psy identity from a
   * personal_sign (NO held key), bind pkHash→evmAddress, and resolve the userId.
   * Idempotent: the WASM register early-returns the existing key when it already
   * has a user, so re-login resolves the SAME pkHash/userId.
   */
  private async registerOrBind(
    evmAddress: string,
    onProgress?: (info: UserIdPollProgress) => void,
  ): Promise<{ userId: string; pkHash: string }> {
    const { pkHash, userId: resolvedUserId } = await this.submitter.registerUser(evmAddress)
    if (!pkHash) throw new Error('Registration did not return a public-key hash.')
    this.registry.register(pkHash, evmAddress)
    const userId =
      resolvedUserId ??
      (await pollUserIdAcrossCheckpoints(pkHash, onProgress, this.pollDeps()))
    return { userId, pkHash }
  }

  // ── login / logout ──────────────────────────────────────────────────────────

  /**
   * Full Mode-A login. TWO wallet prompts on a FRESH login: (1) the auth-message
   * personal_sign that seeds r0/r1+Nostr, and (2) the Mode-A register
   * personal_sign. Thereafter every send/claim pops its OWN per-tx personal_sign.
   *
   * Re-entrant calls while a login is in flight resolve to the in-flight result's
   * session (or throw its error) rather than opening a second prompt.
   */
  async login(opts?: {
    authVersion?: AuthVersion
    onProgress?: (info: UserIdPollProgress) => void
  }): Promise<UnifiedSession> {
    if (this.loginInFlight) {
      throw new Error('A sign-in is already in progress.')
    }
    this.loginInFlight = true
    this.setState({ error: null, phase: 'connecting' })
    try {
      const evmAddress = await this.signer.connect()
      const authVersion = opts?.authVersion ?? this.loadAuthVersion()

      this.setState({ phase: 'preparing-prover' })

      // The auth signature derives PRIVACY material ONLY (r0/r1 + Nostr key) — it
      // is NOT a signing key. v1/v2 differ only in the bytes signed; the
      // derivation is shared (guarded by the golden-vector test).
      let signature: string
      if (authVersion === 'v2') {
        signature = await this.signer.signTypedDataV4(
          buildEip712AuthDomain(),
          EIP712_AUTH_TYPES,
          buildEip712AuthValue(evmAddress),
          evmAddress,
        )
      } else {
        signature = await this.signer.personalSign(buildAuthMessage(evmAddress), evmAddress)
      }
      const material = await deriveKeyMaterialFromSignature(evmAddress, signature)

      const { userId, pkHash } = await this.registerOrBind(evmAddress, (info) => {
        this.setState({ phase: 'finishing-registration' })
        opts?.onProgress?.(info)
      })
      const realmId = this.rpc.realmIdFromUserId(BigInt(userId))
      const shieldAddress = deriveShieldAddressForUser(userId, material.random0, material.random1)

      const session: UnifiedSession = {
        evmAddress,
        userId,
        pkHash,
        realmId,
        shieldAddress,
        random0: material.random0,
        random1: material.random1,
        nostrNpub: material.nostrNpub,
        nostrSecretKeyHex: material.nostrSecretKeyHex,
        authVersion,
      }
      this.setState({ session, phase: 'authed' })
      return session
    } catch (e) {
      this.setState({ error: (e as Error)?.message ?? String(e), phase: 'error' })
      throw e
    } finally {
      this.loginInFlight = false
    }
  }

  /**
   * Log out. Terminates the prover worker (fresh SDK signer registry next login,
   * so prior signer material never lingers) and clears the Mode-A registry (a
   * stale address can never authorize a signature for a new identity). P5
   * trackers tear down off the emitted state change.
   */
  logout(): void {
    this.registry.reset()
    this.runtime.prover.terminate()
    this.setState({ session: null, phase: 'idle', error: null })
  }

  /** Release account-switch watcher. Call when disposing the client. */
  dispose(): void {
    this.unwatchAccounts?.()
    this.unwatchAccounts = null
    this.listeners.clear()
  }
}
