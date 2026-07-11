/**
 * Unified Bridge App — deterministic Psy identity derived from a MetaMask
 * signature. NO seed phrase, NO browser-extension wallet.
 *
 * DESIGN ANCHOR (grounded in the shipped @psy-protocol/psy-sdk):
 *   MetaMask = secp256k1. The SDK's `SignType.SECP256K1Sign` is the
 *   "Classic Wallet (secp256k1)" identity path. We derive a STABLE Psy
 *   child private key from a MetaMask personal_sign signature over a fixed
 *   message, then hand that child key to the SDK's
 *   `PsyWasmWebProverProvider.registerUser(childKey, SignType.SECP256K1Sign)`.
 *
 * Derivation chain (fully deterministic from the EVM account):
 *
 *   message      = AUTH_MESSAGE(evmAddress)              // stable, versioned
 *   signature    = personal_sign(message, evmAddress)    // MetaMask
 *   seed         = keccak256(signature)                  // 32 bytes
 *   psyPrivKey   = keccak256("psy-classic-secp256k1-v1" || seed)  // 32B, the Psy secp256k1 key
 *   r0,r1        = HMAC-SHA256(seed, "psy-privacy-v1-r" || u32(i)) mod p   (i = 0,1)
 *   shieldAddr   = Poseidon(userId, 1337, r0, r1)        // note_owner (needs on-chain userId)
 *   nostrSk      = SHA-256("psy-nostr-v1:" || psyPrivKey || ":" || r0 || ":" || r1)
 *
 * The same EVM account ALWAYS reproduces the same Psy key, r0/r1, shield
 * address and Nostr key — on any browser/device — so recovery needs only
 * MetaMask. We deliberately reuse the wallet/bridge's existing primitives:
 *   - Poseidon shield-address derivation: privacy/lib/crypto.ts deriveShieldAddress
 *   - Nostr key derivation: privacy/lib/crypto.ts deriveNostrKeyFromPsy
 * so on-chain artifacts stay byte-compatible with the rest of the stack.
 */

import { keccak256, getBytes, hexlify } from 'ethers'
import {
  deriveShieldAddress as poseidonShieldAddress,
  deriveNostrKeyFromPsy,
} from '../privacy/crypto'

/** Goldilocks field modulus: 2^64 - 2^32 + 1. r0/r1 must be reduced into it. */
const GOLDILOCKS_MOD = 0xffffffff00000001n

/**
 * Versioned, human-readable auth message. Bumping the version (and ONLY then)
 * rotates every derived Psy identity, so treat it as a hard protocol constant.
 * The message is intentionally free of timestamps/nonces so the signature —
 * and therefore the whole identity — is reproducible forever.
 */
export const AUTH_MESSAGE_VERSION = 'v1'

export function buildAuthMessage(evmAddress: string): string {
  const addr = evmAddress.toLowerCase()
  return [
    'Psy Unified Bridge — Sign in',
    '',
    'Signing this message derives your private Psy account.',
    'It does NOT send a transaction or cost gas.',
    'Anyone who can produce this signature controls the account, so only',
    'sign it on the official Psy app.',
    '',
    `Account: ${addr}`,
    `Version: ${AUTH_MESSAGE_VERSION}`,
  ].join('\n')
}

/* ───────────────────────────────────────────────────────────────────────────
 * Auth versioning (OPT-IN, default unchanged)
 *
 * Two auth signature shapes feed the SAME deterministic derivation
 * (deriveKeyMaterialFromSignature). They differ ONLY in the bytes the user
 * signs; everything downstream of `seed = keccak256(signatureHex)` is shared:
 *
 *   v1  personal_sign over the plain AUTH_MESSAGE string  (the current default)
 *   v2  EIP-712 signTypedData_v4 over a domain-bound typed struct (opt-in)
 *
 * WHY v2 EXISTS: the v1 message is a free-text personal_sign payload, so a
 * malicious site could in principle trick a user into signing that exact string
 * and re-derive their Psy identity elsewhere (the derived Psy key is distinct
 * from the MetaMask key, so ETH funds are never exposed — but the Psy account
 * could be). v2 binds the signature to an EIP-712 domain (name+version+chainId),
 * which makes a blind cross-site signature extremely unlikely to collide with
 * this exact payload.
 *
 * WHY IT IS OPT-IN, NEVER AUTO-MIGRATED: a different signature => a different
 * `seed` => a DIFFERENT Psy private key, r0/r1, shield address and userId. An
 * existing v1 user silently bumped to v2 would land on a brand-new, empty Psy
 * account and lose access to their funded one. So v1 stays the default at every
 * layer and v2 is a clearly-labeled, one-time choice.
 *
 * DETERMINISM (cross-device recovery): every v2 field is a pure function of the
 * EVM address — no nonce, no timestamp, and chainId is PINNED to a constant
 * (NOT read from the wallet). Same account => same typed-data => same signature
 * => same identity on any device/network.
 * ─────────────────────────────────────────────────────────────────────────── */

export type AuthVersion = 'v1' | 'v2'

/** DEFAULT auth version. Stays 'v1' so existing users are never auto-migrated. */
export const DEFAULT_AUTH_VERSION: AuthVersion = 'v1'

/** Label for the EIP-712 (domain-bound) auth variant. */
export const EIP712_AUTH_VERSION = 'v2'

/**
 * PINNED domain chainId. Deliberately a constant, NOT read from the connected
 * wallet: keeping it fixed makes the signed payload identical on every network
 * so seedless cross-device recovery still works. It is a domain LABEL here, not
 * a network assertion (the v2 signature is not a transaction).
 */
export const EIP712_AUTH_CHAIN_ID = 1

/** EIP-712 domain for the v2 auth signature. No `verifyingContract` — name +
 *  version + chainId already bind the payload, and omitting the field avoids
 *  inventing a fake 0x0 contract address. */
export function buildEip712AuthDomain(): {
  name: string
  version: string
  chainId: number
} {
  return {
    name: 'Psy Unified Bridge',
    version: EIP712_AUTH_VERSION, // 'v2' — distinct from the v1 message tag
    chainId: EIP712_AUTH_CHAIN_ID,
  }
}

/** EIP-712 typed-data struct definition (ethers-style: WITHOUT EIP712Domain). */
export const EIP712_AUTH_TYPES = {
  Auth: [
    { name: 'account', type: 'address' },
    { name: 'intent', type: 'string' },
    { name: 'version', type: 'string' },
  ],
} as const

/** Deterministic typed-data value: a pure function of the EVM address. The
 *  address is lowercased to match v1's `addr.toLowerCase()` convention so a
 *  checksummed-vs-lowercase address can never fork the derived identity. */
export function buildEip712AuthValue(evmAddress: string): {
  account: string
  intent: string
  version: string
} {
  return {
    account: evmAddress.toLowerCase(),
    intent: 'Derive my private Psy account',
    version: EIP712_AUTH_VERSION,
  }
}

export interface DerivedPsyKeyMaterial {
  /** Lowercased EVM address the identity is bound to. */
  evmAddress: string
  /** 32-byte Psy secp256k1 private key, 0x-prefixed hex. Feed to the SDK. */
  psyPrivateKeyHex: string
  /** Privacy scalars (decimal strings) shared by shield + Nostr derivation. */
  random0: string
  random1: string
  /** Deterministic Nostr keypair for NIP-59 note delivery. */
  nostrSecretKeyHex: string
  nostrNpub: string
  nostrNsec: string
}

function hmacKey(seed: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey('raw', seed, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign'])
}

function u32be(i: number): Uint8Array {
  const b = new Uint8Array(4)
  new DataView(b.buffer).setUint32(0, i >>> 0, false)
  return b
}

function bytesToBigIntBE(b: Uint8Array): bigint {
  let v = 0n
  for (const byte of b) v = (v << 8n) | BigInt(byte)
  return v
}

/** Derive r_i = HMAC-SHA256(seed, "psy-privacy-v1-r" || u32be(i)) mod p. */
async function deriveScalar(seed: Uint8Array, index: number): Promise<bigint> {
  const key = await hmacKey(seed)
  const prefix = new TextEncoder().encode('psy-privacy-v1-r')
  const idx = u32be(index)
  const msg = new Uint8Array(prefix.length + idx.length)
  msg.set(prefix, 0)
  msg.set(idx, prefix.length)
  const mac = new Uint8Array(await crypto.subtle.sign('HMAC', key, msg))
  // Reduce the full 256-bit MAC into the Goldilocks field.
  return bytesToBigIntBE(mac) % GOLDILOCKS_MOD
}

/** keccak256(label || data) → 0x-hex (32 bytes). */
function keccakLabeled(label: string, data: Uint8Array): string {
  const lab = new TextEncoder().encode(label)
  const buf = new Uint8Array(lab.length + data.length)
  buf.set(lab, 0)
  buf.set(data, lab.length)
  return keccak256(buf)
}

/**
 * Pure, testable derivation from a MetaMask signature. Given the same
 * signature it ALWAYS returns the same key material.
 */
export async function deriveKeyMaterialFromSignature(
  evmAddress: string,
  signatureHex: string,
): Promise<DerivedPsyKeyMaterial> {
  const seedHex = keccak256(signatureHex) // 32-byte deterministic seed
  const seed = getBytes(seedHex)

  const psyPrivateKeyHex = keccakLabeled('psy-classic-secp256k1-v1', seed)

  const r0 = await deriveScalar(seed, 0)
  const r1 = await deriveScalar(seed, 1)
  const random0 = r0.toString()
  const random1 = r1.toString()

  // Reuse the bridge's existing Nostr derivation so delivery stays
  // byte-compatible with the rest of the privacy stack.
  const nostr = await deriveNostrKeyFromPsy(psyPrivateKeyHex, random0, random1)

  return {
    evmAddress: evmAddress.toLowerCase(),
    psyPrivateKeyHex,
    random0,
    random1,
    nostrSecretKeyHex: hexlify(nostr.sk),
    nostrNpub: nostr.npub,
    nostrNsec: nostr.nsec,
  }
}

/**
 * Shield (note_owner) address. Needs the on-chain userId, so it's computed
 * AFTER registration. Reuses the shared Poseidon derivation verbatim.
 * Returns the canonical colon-joined 4×u64 hex form, or null if inputs
 * are malformed.
 */
export function deriveShieldAddressForUser(
  userId: string,
  random0: string,
  random1: string,
): string | null {
  const limbs = poseidonShieldAddress(userId, random0, random1)
  if (!limbs) return null
  return limbs.map((n) => '0x' + n.toString(16).padStart(16, '0')).join(':')
}

export const _internal = { GOLDILOCKS_MOD, deriveScalar, keccakLabeled }
