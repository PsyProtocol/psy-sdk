/**
 * Mode-A signature seam — the load-bearing byte-layout + secp256k1 recovery
 * logic, ISOLATED from MetaMask and the prover worker so it is pure and
 * unit-testable (see tests/mode-a-sig.test.ts) WITHOUT the rebuilt SDK.
 *
 * Mode-A = the web wallet's per-tx auth: the secp256k1 key NEVER leaves
 * MetaMask. Each contract call is authorized by an `eth_sign` over the Psy
 * session sighash (a RAW 32-byte prehash — NO EIP-191 prefix), and the proven
 * in-circuit secp256k1 verifier checks that signature. There is NO circuit
 * change: the circuit already verifies a raw `sign_prehash`, which is exactly
 * `eth_sign` semantics.
 *
 * THE EXTERNAL-SIGNATURE PAYLOAD (what `exec_contract_call_with_external_signature`
 * expects as `signature_hex`) is a 97-byte blob:
 *
 *     compressedPubkey(33) ‖ r(32) ‖ s(32)   ==>  33 + 32 + 32 === 97
 *
 * The 33-byte compressed pubkey is RECOVERED from the (sighash, 65-byte
 * eth_sign signature) pair — MetaMask `eth_sign` returns r‖s‖v, and v lets us
 * recover the public key with no separate "which account" lookup. We then
 * compress it to 33 bytes.
 *
 * RUNTIME ASSUMPTION (documented, not silently resolved): the in-circuit
 * verifier is assumed to accept a low-S–normalized signature. We normalize to
 * low-S (BIP-62) here so the high-S malleable half can't be rejected. Confirm
 * against the SDK's secp256k1 verifier before E2E.
 */

import { Signature, SigningKey, getBytes, hashMessage } from 'ethers'

/** secp256k1 curve order n (BIP-62 low-S threshold is n/2). */
const SECP256K1_N = BigInt(
  '0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141',
)
const SECP256K1_HALF_N = SECP256K1_N >> 1n

const COMPRESSED_PUBKEY_BYTES = 33
const SCALAR_BYTES = 32
/** 33 + 32 + 32. Asserted at runtime in assembleExternalSignaturePayload. */
export const EXTERNAL_SIGNATURE_BYTES =
  COMPRESSED_PUBKEY_BYTES + SCALAR_BYTES + SCALAR_BYTES // === 97

function strip0x(hex: string): string {
  return hex.startsWith('0x') || hex.startsWith('0X') ? hex.slice(2) : hex
}

function assertByteLen(label: string, hex: string, want: number): string {
  const bare = strip0x(hex).toLowerCase()
  if (!/^[0-9a-f]*$/.test(bare)) {
    throw new Error(`Mode-A ${label}: not hex (${hex})`)
  }
  if (bare.length !== want * 2) {
    throw new Error(
      `Mode-A ${label}: expected ${want} bytes (${want * 2} hex chars), got ${bare.length / 2}`,
    )
  }
  return bare
}

/**
 * Guard the byte-exact contract that Mode-A's sighash is a RAW 32-byte prehash
 * — never an EIP-191 (`\x19Ethereum Signed Message:\n32` ‖ digest) envelope.
 * Wrapping the digest would change the bytes the circuit's secp256k1 verifier
 * hashes over and the proof would NOT verify. This asserts the length and
 * shape; the caller must pass the `get_sig_hash` output verbatim (NOT a
 * personal_sign of it).
 */
export function assertRawPrehash(sigHash32Hex: string): string {
  return '0x' + assertByteLen('sigHash (raw prehash)', sigHash32Hex, 32)
}

/**
 * Enforce low-S (BIP-62) on a RAW 65-byte `eth_sign` blob, returning a
 * canonical ethers `Signature`. This MUST normalize at the byte level BEFORE
 * handing to `Signature.from`, because ethers v6 `Signature.from` REJECTS a
 * non-canonical (high-S) signature outright ("non-canonical s") rather than
 * accepting and exposing it — so we cannot normalize a parsed `Signature`, we
 * have to fix the bytes first.
 *
 * The secp256k1 signature is malleable (s and n-s both satisfy the ECDSA
 * equation), and an in-circuit verifier commonly REQUIRES the canonical low-S
 * half. MetaMask `eth_sign` is expected to emit low-S already, but we normalize
 * defensively so a non-canonical blob can't be rejected downstream.
 *
 * NOTE: flipping s also flips the recovery parity, so v (27↔28 / 0↔1) is flipped
 * to stay consistent and keep recovery yielding the right key.
 */
export function normalizeLowS(sig65Hex: string): Signature {
  const bare = assertByteLen('eth_sign signature', sig65Hex, 65)
  const r = '0x' + bare.slice(0, 64)
  const sHex = bare.slice(64, 128)
  let s = BigInt('0x' + sHex)
  let v = parseInt(bare.slice(128, 130), 16) // 27/28 (or 0/1)
  if (s > SECP256K1_HALF_N) {
    s = SECP256K1_N - s
    // Flip recovery parity. Handle both legacy {27,28} and raw {0,1} encodings.
    if (v === 27 || v === 28) v = v === 27 ? 28 : 27
    else v = v ^ 1
  }
  return Signature.from({
    r,
    s: '0x' + s.toString(16).padStart(64, '0'),
    v,
  })
}

/**
 * Recover the 33-byte COMPRESSED secp256k1 public key from a raw 32-byte
 * sighash and a 65-byte `eth_sign` signature (r‖s‖v). Applies low-S
 * normalization first so the recovered key matches the normalized (r,s) we will
 * ship in the payload.
 *
 * Returns bare lowercase hex (no `0x`), 33 bytes.
 */
export function recoverCompressedPubkey(
  sigHash32Hex: string,
  sig65Hex: string,
): string {
  const digest = assertRawPrehash(sigHash32Hex)
  const sig = normalizeLowS(sig65Hex)
  // SigningKey.recoverPublicKey wants the digest + the (normalized) signature.
  const uncompressed = SigningKey.recoverPublicKey(getBytes(digest), sig)
  const compressed = SigningKey.computePublicKey(uncompressed, /*compressed*/ true)
  return assertByteLen('recovered compressed pubkey', compressed, COMPRESSED_PUBKEY_BYTES)
}

/**
 * Split a 65-byte `eth_sign` signature into low-S-normalized (r, s) bare-hex
 * scalars. Exposed so the orchestrator can assemble the payload from the SAME
 * normalized signature it recovered the pubkey from.
 */
export function normalizedRS(sig65Hex: string): { r: string; s: string } {
  const sig = normalizeLowS(sig65Hex)
  return {
    r: assertByteLen('r', sig.r, SCALAR_BYTES),
    s: assertByteLen('s', sig.s, SCALAR_BYTES),
  }
}

/**
 * Assemble the 97-byte external-signature payload the rebuilt SDK's
 * `exec_contract_call_with_external_signature` expects:
 *
 *     compressedPubkey(33) ‖ r(32) ‖ s(32)
 *
 * Each component is length-checked; the total is asserted to be exactly
 * EXTERNAL_SIGNATURE_BYTES (97). Returns `0x`-prefixed hex.
 */
export function assembleExternalSignaturePayload(
  compressedPubkeyHex: string,
  r: string,
  s: string,
): string {
  const pk = assertByteLen('compressed pubkey', compressedPubkeyHex, COMPRESSED_PUBKEY_BYTES)
  const rr = assertByteLen('r', r, SCALAR_BYTES)
  const ss = assertByteLen('s', s, SCALAR_BYTES)
  const payload = pk + rr + ss
  // Byte contract enforced in code (not prose): 33 + 32 + 32 === 97.
  if (payload.length !== EXTERNAL_SIGNATURE_BYTES * 2) {
    throw new Error(
      `Mode-A external signature: expected ${EXTERNAL_SIGNATURE_BYTES} bytes, got ${payload.length / 2}`,
    )
  }
  return '0x' + payload
}

/**
 * One-shot: from a raw sighash + a 65-byte eth_sign signature, build the
 * 97-byte external-signature payload (recover → compress → assemble), using a
 * single low-S normalization so the shipped (r,s) match the key recovery.
 */
export function buildExternalSignaturePayload(
  sigHash32Hex: string,
  sig65Hex: string,
): { payload: string; compressedPubkey: string } {
  const compressedPubkey = recoverCompressedPubkey(sigHash32Hex, sig65Hex)
  const { r, s } = normalizedRS(sig65Hex)
  const payload = assembleExternalSignaturePayload('0x' + compressedPubkey, '0x' + r, '0x' + s)
  return { payload, compressedPubkey: '0x' + compressedPubkey }
}

/**
 * Recover the 33-byte COMPRESSED pubkey from a `personal_sign` (EIP-191)
 * signature. Identical to [`recoverCompressedPubkey`] except the recovery digest
 * is the EIP-191 envelope `keccak256("\x19Ethereum Signed Message:\n32" ||
 * sighash)` (ethers `hashMessage` over the raw 32 sighash bytes) — the digest
 * MetaMask actually signed — rather than the raw sighash. The (r,s) scalars are
 * unchanged; only the message the pubkey is recovered against differs.
 */
export function recoverCompressedPubkeyPersonalSign(
  sigHash32Hex: string,
  sig65Hex: string,
): string {
  const rawDigest = assertRawPrehash(sigHash32Hex)
  // EIP-191 digest over the RAW 32 sighash bytes — matches MetaMask personal_sign
  // and the in-circuit keccak in EthPersonalSignSecp256K1Gadget.
  const eip191Digest = hashMessage(getBytes(rawDigest))
  const sig = normalizeLowS(sig65Hex)
  const uncompressed = SigningKey.recoverPublicKey(getBytes(eip191Digest), sig)
  const compressed = SigningKey.computePublicKey(uncompressed, /*compressed*/ true)
  return assertByteLen('recovered compressed pubkey', compressed, COMPRESSED_PUBKEY_BYTES)
}

/**
 * One-shot for MODE-A `personal_sign`: from a raw sighash + a 65-byte
 * `personal_sign` signature, build the 97-byte external-signature payload. The
 * shipped `(r,s)` are the SAME signature scalars; the compressed pubkey is
 * recovered against the EIP-191 digest (see [`recoverCompressedPubkeyPersonalSign`]).
 */
export function buildExternalSignaturePayloadPersonalSign(
  sigHash32Hex: string,
  sig65Hex: string,
): { payload: string; compressedPubkey: string } {
  const compressedPubkey = recoverCompressedPubkeyPersonalSign(sigHash32Hex, sig65Hex)
  const { r, s } = normalizedRS(sig65Hex)
  const payload = assembleExternalSignaturePayload('0x' + compressedPubkey, '0x' + r, '0x' + s)
  return { payload, compressedPubkey: '0x' + compressedPubkey }
}
