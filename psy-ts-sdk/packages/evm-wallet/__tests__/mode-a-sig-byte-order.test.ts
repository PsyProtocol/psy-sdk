/**
 * REGRESSION GUARD — Mode-A signature BYTE-ORDER (endianness) contract.
 *
 * A past bug reversed the session sighash to little-endian BEFORE signing. The
 * in-circuit secp256k1 / EIP-191 verifier actually hashes+verifies the
 * BIG-ENDIAN sighash VERBATIM — this is proven by the Rust golden test
 * `test_eth_personal_sign_gadget` in `psy_common_circuit` (the source of truth
 * for the BE-verbatim convention). Because of that, the submitter
 * (mode-a-submitter.ts execPublicContractCall / claimBatch / registerUser) MUST:
 *
 *   1. hand `assertRawPrehash(sigHash)` to the wallet VERBATIM (no reversal), and
 *   2. call `buildExternalSignaturePayloadPersonalSign` with that SAME digest,
 *
 * so the pubkey is recovered against the EIP-191 hash of the digest AS-GIVEN.
 *
 * These tests pin the byte-exact behavior so a future LE reversal (in either
 * `assertRawPrehash` or the personal-sign recovery) turns RED here instead of
 * silently producing proofs that no longer verify on-chain.
 *
 * Pure — no rebuilt SDK, no MetaMask. Runs under the package's jest setup:
 *   jest __tests__/mode-a-sig-byte-order.test.ts
 */
import { test, expect, describe } from '@jest/globals'
import { SigningKey, getBytes, hashMessage } from 'ethers'
import {
  EXTERNAL_SIGNATURE_BYTES,
  assertRawPrehash,
  normalizedRS,
  buildExternalSignaturePayloadPersonalSign,
} from '../src/session/mode-a-sig'

// Fixed key for determinism (a well-known secp256k1 test scalar).
const PRIV = '0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d'
const KEY = new SigningKey(PRIV)
const EXPECTED_COMPRESSED = SigningKey.computePublicKey(KEY.publicKey, true).toLowerCase()

// An ASYMMETRIC (non-palindromic) raw 32-byte prehash: bytes 0x00..0x1f.
// Asymmetry is what makes a byte-reversal observable — a palindromic digest
// (e.g. 0x1111..) would hide an LE regression.
const DIGEST =
  '0x' +
  Array.from({ length: 32 }, (_, i) => i.toString(16).padStart(2, '0')).join('')

/** Reverse the byte order of a 0x + 64-hex string (the LE twin of the digest). */
function reverseBytes32(hex0x: string): string {
  const bare = hex0x.slice(2)
  const bytes = bare.match(/../g)!
  return '0x' + bytes.reverse().join('')
}

/**
 * MetaMask `personal_sign` semantics: sign the EIP-191 envelope
 * keccak256("\x19Ethereum Signed Message:\n32" ‖ digest) over the RAW 32
 * sighash bytes. ethers `hashMessage(getBytes(digest))` reproduces that digest.
 */
function personalSign65(digest32: string): string {
  return KEY.sign(getBytes(hashMessage(getBytes(digest32)))).serialized
}

describe('mode-a-sig: byte-order / endianness regression guard', () => {
  test('assertRawPrehash returns the 32-byte digest VERBATIM (no reversal/mutation)', () => {
    // Same bytes back, in the same order — NOT the little-endian twin.
    expect(assertRawPrehash(DIGEST)).toBe(DIGEST)
    expect(assertRawPrehash(DIGEST)).not.toBe(reverseBytes32(DIGEST))
    // Idempotent + order-preserving for an asymmetric input.
    expect(assertRawPrehash(assertRawPrehash(DIGEST))).toBe(DIGEST)
  })

  test('assertRawPrehash rejects wrong-length input', () => {
    expect(() => assertRawPrehash('0x' + '00'.repeat(31))).toThrow() // 31 bytes
    expect(() => assertRawPrehash('0x' + '00'.repeat(33))).toThrow() // 33 bytes
    expect(() => assertRawPrehash('0x')).toThrow() // 0 bytes
    expect(() => assertRawPrehash('0xzz')).toThrow() // not hex
  })

  test('buildExternalSignaturePayloadPersonalSign: 97-byte pubkey‖r‖s layout', () => {
    const sig65 = personalSign65(DIGEST)
    const { payload, compressedPubkey } = buildExternalSignaturePayloadPersonalSign(DIGEST, sig65)

    // (a) exactly 97 bytes = 33 (compressed pubkey) + 32 (r) + 32 (s).
    expect(payload.startsWith('0x')).toBe(true)
    expect((payload.length - 2) / 2).toBe(EXTERNAL_SIGNATURE_BYTES)
    expect(EXTERNAL_SIGNATURE_BYTES).toBe(97)

    // layout is compressedPubkey ‖ r ‖ s, in that order.
    const { r, s } = normalizedRS(sig65)
    expect(payload.slice(2)).toBe(compressedPubkey.slice(2) + r + s)
    expect((compressedPubkey.length - 2) / 2).toBe(33)
  })

  test('recovers the pubkey from the EIP-191 hash of the digest AS-GIVEN (BE-verbatim)', () => {
    const sig65 = personalSign65(DIGEST)
    const { compressedPubkey } = buildExternalSignaturePayloadPersonalSign(DIGEST, sig65)
    // The signer signed personal_sign(DIGEST); recovery against the SAME digest
    // must round-trip to the signing key. This is the BE-verbatim contract that
    // the Rust golden test `test_eth_personal_sign_gadget` enforces in-circuit.
    expect(compressedPubkey.toLowerCase()).toBe(EXPECTED_COMPRESSED)
  })

  test('REGRESSION: a byte-REVERSED digest recovers a DIFFERENT pubkey (the LE bug guard)', () => {
    const sig65 = personalSign65(DIGEST)
    const reversed = reverseBytes32(DIGEST)
    expect(reversed).not.toBe(DIGEST) // sanity: the digest is asymmetric

    const original = buildExternalSignaturePayloadPersonalSign(DIGEST, sig65).compressedPubkey
    const leTwin = buildExternalSignaturePayloadPersonalSign(reversed, sig65).compressedPubkey

    // Feeding the little-endian twin yields a DIFFERENT recovered pubkey, proving
    // the function is byte-order-sensitive. Therefore the submitter MUST pass the
    // sighash VERBATIM — if it reversed to LE (the old bug), the on-chain
    // secp256k1 verifier would recover the wrong key and the proof would fail.
    expect(original.toLowerCase()).toBe(EXPECTED_COMPRESSED)
    expect(leTwin.toLowerCase()).not.toBe(original.toLowerCase())
  })
})
