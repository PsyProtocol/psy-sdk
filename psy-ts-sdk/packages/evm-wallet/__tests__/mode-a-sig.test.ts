/**
 * Byte-layout + secp256k1-recovery vectors for the Mode-A signature seam.
 * The ONLY part of the Mode-A wiring verifiable without the rebuilt SDK or a
 * browser — runs under `bun test` in <1s. (The SDK call + MetaMask eth_sign are
 * exercised only in browser E2E.)
 *
 *   bun test src/unified/mode-a-sig.test.ts
 */
import { test, expect, describe } from '@jest/globals'
import { SigningKey, getBytes } from 'ethers'
import {
  EXTERNAL_SIGNATURE_BYTES,
  assertRawPrehash,
  normalizeLowS,
  recoverCompressedPubkey,
  normalizedRS,
  assembleExternalSignaturePayload,
  buildExternalSignaturePayload,
} from '../src/session/mode-a-sig'

const SECP_N = BigInt(
  '0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141',
)
const HALF_N = SECP_N >> 1n

// Fixed key for determinism (a well-known secp256k1 test scalar).
const PRIV = '0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d'
const KEY = new SigningKey(PRIV)
const EXPECTED_COMPRESSED = SigningKey.computePublicKey(KEY.publicKey, true).toLowerCase()

const DIGEST = '0x' + '11'.repeat(32) // a raw 32-byte prehash (NOT EIP-191 wrapped)

/** eth_sign semantics: sign the RAW digest (no prefix), return the 65-byte r‖s‖v blob. */
function ethSign65(digest32: string): string {
  return KEY.sign(getBytes(digest32)).serialized // 0x + r(32)+s(32)+v(1)
}

describe('mode-a-sig: byte layout + recovery', () => {
  test('EXTERNAL_SIGNATURE_BYTES === 97 (33 + 32 + 32)', () => {
    expect(EXTERNAL_SIGNATURE_BYTES).toBe(97)
  })

  test('assertRawPrehash accepts exactly 32 bytes, rejects others', () => {
    expect(assertRawPrehash(DIGEST)).toBe(DIGEST)
    expect(() => assertRawPrehash('0x' + '11'.repeat(31))).toThrow()
    expect(() => assertRawPrehash('0x' + '11'.repeat(33))).toThrow()
    expect(() => assertRawPrehash('0xnothex')).toThrow()
  })

  test('recoverCompressedPubkey round-trips to the signing key', () => {
    const recovered = recoverCompressedPubkey(DIGEST, ethSign65(DIGEST))
    expect('0x' + recovered).toBe(EXPECTED_COMPRESSED)
  })

  test('payload is 0x-prefixed, exactly 97 bytes, and is pubkey‖r‖s', () => {
    const sig = ethSign65(DIGEST)
    const { payload, compressedPubkey } = buildExternalSignaturePayload(DIGEST, sig)
    expect(payload.startsWith('0x')).toBe(true)
    expect((payload.length - 2) / 2).toBe(97)
    const { r, s } = normalizedRS(sig)
    expect(payload.slice(2)).toBe(compressedPubkey.slice(2) + r + s)
  })

  test('assembleExternalSignaturePayload rejects wrong-length components', () => {
    const pk = '0x' + '02' + 'aa'.repeat(32) // 33 bytes
    const sc = '0x' + 'bb'.repeat(32) // 32 bytes
    expect(assembleExternalSignaturePayload(pk, sc, sc).length).toBe(2 + 97 * 2)
    expect(() => assembleExternalSignaturePayload('0x' + 'aa'.repeat(32), sc, sc)).toThrow() // 32-byte pk
    expect(() => assembleExternalSignaturePayload(pk, '0x' + 'bb'.repeat(31), sc)).toThrow() // short r
  })

  test('normalizeLowS forces low-S and a high-S blob still recovers the key', () => {
    const sig = KEY.sign(getBytes(DIGEST)) // ethers emits low-S
    // Craft the malleable high-S twin: s -> n-s, and flip v parity to stay recoverable.
    const highS = SECP_N - BigInt(sig.s)
    const flippedV = sig.v === 27 ? 28 : 27
    const highSBlob =
      '0x' +
      sig.r.slice(2) +
      highS.toString(16).padStart(64, '0') +
      flippedV.toString(16).padStart(2, '0')

    const norm = normalizeLowS(highSBlob)
    expect(BigInt(norm.s) <= HALF_N).toBe(true) // normalized back to low-S
    // Recovery from the high-S blob still yields the original key.
    expect('0x' + recoverCompressedPubkey(DIGEST, highSBlob)).toBe(EXPECTED_COMPRESSED)
  })
})
