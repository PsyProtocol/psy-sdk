/**
 * Mode-A submit orchestrator — the per-tx wallet-auth flow.
 *
 *   getSigHash(pkHash, callData) -> 32-byte sighash
 *     -> personal_sign(sighash)  (RAW prehash; key NEVER leaves the wallet)
 *       -> recover compressed pubkey + low-S (r,s)  (mode-a-sig.ts)
 *         -> execContractCallWithExternalEthPersonalSignature(pkHash, callData, payload)
 *           -> tx hash
 *
 * Ported from the app's unified/mode-a.ts. The ONLY change: callProver and
 * ethPersonalSign are injected (from the runtime's ProverEngine + EvmSigner)
 * instead of module imports. The secp256k1 private key is NEVER held — every
 * register/send/claim is authorized by a personal_sign popup; the Mode-A
 * identity IS the wallet account key (pkHash = hash of its secp256k1 pubkey).
 *
 * DISPATCH: the Mode-A methods are camelCase provider-instance methods on
 * PsyWasmWebProverProvider; the worker dispatches provider[method] by name, so
 * callProver(label, method, args) reaches them with no worker/codec change.
 */

import type { ProverEngine } from '../prover/engine'
import type { EvmSigner } from '../evm/signer'
import type { ContractCallArgs, ClaimBatchItem } from '../psy/types'
import {
  buildExternalSignaturePayloadPersonalSign,
  assertRawPrehash,
} from './mode-a-sig'

/** software_defined_call default — byte-identical to the ops layer's
 *  EMPTY_SIGN_DATA so the callData Mode-A signs over is the SAME envelope the
 *  chain proves. */
const EMPTY_SIGN_DATA = { inputs: [] as number[] }

/** Build the ContractCallData wrapper — MUST match the ops layer's wrapCallData
 *  exactly so getSigHash signs over the identical bytes exec_* proves. */
function wrapCallData(calls: ContractCallArgs[]): {
  contract_calls: ContractCallArgs[]
  software_defined_call: { inputs: number[] }
} {
  return { contract_calls: calls, software_defined_call: EMPTY_SIGN_DATA }
}

type CallData = ReturnType<typeof wrapCallData>

export interface ModeAResult {
  txHash: string
  /** The compressed secp256k1 pubkey recovered from the personal_sign signature
   *  (0x-hex, 33 bytes). */
  recoveredPubkey: string
}

/** Recovery-only digest Mode-A registration personal_signs to recover the
 *  wallet's secp256k1 PUBLIC key. The register WASM method binds the all-zero
 *  hash internally (registration consumes no session sighash); ANY well-formed
 *  32-byte prehash recovers the same pubkey, so we sign a FIXED 32-byte digest —
 *  stable so the popup is identical every login. The bytes are the ASCII tag
 *  "psy-mode-a-register-v1" right-zero-padded to 32 bytes. */
const REGISTER_RECOVERY_DIGEST =
  '0x' + '7073792d6d6f64652d612d72656769737465722d7631'.padEnd(64, '0')

export class ModeASubmitter {
  constructor(
    private readonly prover: Pick<ProverEngine, 'callProver'>,
    private readonly signer: Pick<EvmSigner, 'ethPersonalSign'>,
  ) {}

  // ─── typed wrappers over the rebuilt-SDK provider methods ───────────────────

  private getSigHash(pkHash: string, callData: CallData): Promise<string> {
    return this.prover.callProver<string>('getSigHash', 'getSigHash', [pkHash, callData])
  }

  private execWithExternalSignature(
    pkHash: string,
    callData: CallData,
    signatureHex: string,
  ): Promise<string> {
    return this.prover.callProver<string>(
      'execContractCallWithExternalEthPersonalSignature',
      'execContractCallWithExternalEthPersonalSignature',
      [pkHash, callData, signatureHex],
    )
  }

  private getClaimSigHash(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
    return this.prover.callProver<string>('getClaimSigHash', 'getClaimSigHash', [pkHash, claims])
  }

  private claimWithExternalSignature(
    pkHash: string,
    claims: ClaimBatchItem[],
    signatureHex: string,
  ): Promise<string> {
    return this.prover.callProver<string>(
      'claimBatchWithExternalEthPersonalSignature',
      'claimBatchWithExternalEthPersonalSignature',
      [pkHash, claims, signatureHex],
    )
  }

  private registerWithExternalSignature(
    publicKeyHex: string,
    signatureHex: string,
  ): Promise<{ pk_hash: string; user_id: string | null }> {
    return this.prover.callProver<{ pk_hash: string; user_id: string | null }>(
      'registerUserWithExternalEthPersonalSignature',
      'registerUserWithExternalEthPersonalSignature',
      [publicKeyHex, signatureHex],
    )
  }

  // ─── public submit flows ────────────────────────────────────────────────────

  /**
   * Submit one or more public contract calls under Mode-A: prove + auth via a
   * per-tx personal_sign, the secp256k1 key never leaving the wallet.
   */
  async execPublicContractCall(
    pkHash: string,
    calls: ContractCallArgs | ContractCallArgs[],
    ethAddress: string,
  ): Promise<ModeAResult> {
    const list = Array.isArray(calls) ? calls : [calls]
    if (list.length === 0) throw new Error('No contract calls to submit')
    const callData = wrapCallData(list)

    // 1. Ask the prover for the raw 32-byte session sighash — PRIMES the session;
    //    the matching submit reuses it (stale-state guard).
    const sigHash = await this.getSigHash(pkHash, callData)
    const digest = assertRawPrehash(sigHash)

    // 2. personal_sign the digest (returns 65-byte r‖s‖v). >>> POPUP <<<
    const sig65 = await this.signer.ethPersonalSign(digest, ethAddress)

    // 3. Recover the compressed pubkey (EIP-191 digest) + low-S (r,s), assemble
    //    the 97-byte external-signature payload.
    const { payload, compressedPubkey } = buildExternalSignaturePayloadPersonalSign(digest, sig65)

    // 4. Submit; the in-circuit EIP-191 secp256k1 verifier recomputes
    //    keccak(prefix‖sighash) and checks the signature.
    const txHash = await this.execWithExternalSignature(pkHash, callData, payload)

    return {
      txHash: typeof txHash === 'string' ? txHash : String(txHash),
      recoveredPubkey: compressedPubkey,
    }
  }

  /**
   * Submit a claim batch under Mode-A: prove the batch, then authorize it with a
   * per-tx personal_sign over the claim-session sighash.
   */
  async claimBatch(
    pkHash: string,
    claims: ClaimBatchItem[],
    ethAddress: string,
  ): Promise<ModeAResult> {
    if (claims.length === 0) throw new Error('No claim items to submit')

    const sigHash = await this.getClaimSigHash(pkHash, claims)
    const digest = assertRawPrehash(sigHash)
    const sig65 = await this.signer.ethPersonalSign(digest, ethAddress)
    const { payload, compressedPubkey } = buildExternalSignaturePayloadPersonalSign(digest, sig65)
    const txHash = await this.claimWithExternalSignature(pkHash, claims, payload)

    return {
      txHash: typeof txHash === 'string' ? txHash : String(txHash),
      recoveredPubkey: compressedPubkey,
    }
  }

  /**
   * Mode-A registration: recover the wallet's secp256k1 PUBLIC key from ONE
   * personal_sign, then register that public key as a Psy identity with NO held
   * private key. Returns the pkHash + resolved user_id (null until it lands
   * on-chain — the caller polls).
   */
  async registerUser(
    ethAddress: string,
  ): Promise<{ pkHash: string; userId: string | null; recoveredPubkey: string }> {
    const digest = assertRawPrehash(REGISTER_RECOVERY_DIGEST)
    const sig65 = await this.signer.ethPersonalSign(digest, ethAddress)
    const { payload, compressedPubkey } = buildExternalSignaturePayloadPersonalSign(digest, sig65)
    const res = await this.registerWithExternalSignature(compressedPubkey, payload)
    return {
      pkHash: res.pk_hash,
      userId: res.user_id ?? null,
      recoveredPubkey: compressedPubkey,
    }
  }
}
