/**
 * REGRESSION GUARD — Mode-A registry KEY NORMALIZATION contract.
 *
 * The ops layer takes a pkHash but NOT the EVM address, and looks the address up
 * in ModeARegistry to decide the submit path:
 *
 *   - pkHash IS registered  → EVM identity → authorize via personal_sign (a
 *     MetaMask popup pops for every op; no secp256k1 key is ever held).
 *   - pkHash is NOT registered → held-key operator → legacy held-key submit path.
 *
 * The load-bearing invariant behind the session-binding fix: a pkHash bound at
 * LOGIN must resolve at OP time even when the two surfaces spell the same logical
 * key differently (0x-prefixed vs bare, upper vs lower case). If bind-key and
 * lookup-key ever disagree, `ethAddressForPkHash` returns null and the op is
 * silently routed to the held-key path with NO MetaMask popup — the exact
 * no-popup bug this guards against.
 *
 * `norm()` (strip leading 0x + lowercase) is the single seam that keeps
 * bind-key === lookup-key. These tests pin that property so any regression in
 * norm() — used by BOTH register and ethAddressForPkHash — turns RED here.
 *
 * Pure — no rebuilt SDK, no MetaMask. Runs under the package's jest setup:
 *   jest __tests__/mode-a-registry.test.ts
 */
import { test, expect, describe, beforeEach } from '@jest/globals'
import { ModeARegistry } from '../src/session/mode-a-registry'

// A logical pkHash expressed in its canonical (bare lowercase) form, plus the
// same logical key in the OTHER surface forms register/lookup can receive.
const CANON = 'abcd0123456789abcdef00112233445566778899aabbccddeeff001122334455'
const WITH_0X = '0x' + CANON
const UPPER = CANON.toUpperCase()
const WITH_0X_UPPER = '0X' + UPPER
const MIXED = '0xAbCd0123456789ABCDEFabcdef00112233445566778899AABBCCDDEEFF00112233' // unrelated-looking mixed key

// EVM account that authorizes the identity, given mixed-case to prove storage
// lowercases it.
const ETH_ADDR = '0xAbCdEf0123456789AbCdEf0123456789AbCdEf01'
const ETH_ADDR_LC = ETH_ADDR.toLowerCase()

describe('ModeARegistry: key-normalization / bind-key===lookup-key regression guard', () => {
  let reg: ModeARegistry
  beforeEach(() => {
    reg = new ModeARegistry()
  })

  test('norm() consistency: register(0x+UPPER) resolves via bare-lowercase lookup', () => {
    // Bind at "login" with a 0x-prefixed, UPPER-case surface form...
    reg.register(WITH_0X_UPPER, ETH_ADDR)
    // ...and look up at "op time" with the bare lowercase twin of the SAME
    // logical key. This is the exact property whose violation caused the
    // no-popup bug: a mismatch would return null → held-key path.
    expect(reg.ethAddressForPkHash(CANON)).toBe(ETH_ADDR_LC)
    expect(reg.isModeAIdentity(CANON)).toBe(true)
  })

  test('norm() consistency: every surface form of the same key resolves identically', () => {
    reg.register(CANON, ETH_ADDR)
    // 0x/bare and upper/lower are all the SAME logical key → all must resolve.
    for (const form of [CANON, WITH_0X, UPPER, WITH_0X_UPPER]) {
      expect(reg.ethAddressForPkHash(form)).toBe(ETH_ADDR_LC)
      expect(reg.isModeAIdentity(form)).toBe(true)
    }
  })

  test('register stores the eth address LOWERCASED', () => {
    reg.register(WITH_0X, ETH_ADDR) // ETH_ADDR is mixed-case
    expect(reg.ethAddressForPkHash(WITH_0X)).toBe(ETH_ADDR_LC)
  })

  test('ethAddressForPkHash returns null for an UNREGISTERED pkHash (held-key path)', () => {
    reg.register(CANON, ETH_ADDR)
    // A different logical key was never bound → null → legacy held-key operator.
    const other = '9999' + CANON.slice(4)
    expect(reg.ethAddressForPkHash(other)).toBeNull()
    expect(reg.isModeAIdentity(other)).toBe(false)
  })

  test('isModeAIdentity reflects registration true/false (surface-form independent)', () => {
    expect(reg.isModeAIdentity(CANON)).toBe(false)
    reg.register(MIXED, ETH_ADDR)
    expect(reg.isModeAIdentity(MIXED)).toBe(true)
    expect(reg.isModeAIdentity(MIXED.toLowerCase())).toBe(true)
    expect(reg.isModeAIdentity('0X' + MIXED.slice(2).toUpperCase())).toBe(true)
  })

  test('reset() clears ALL bindings (so a stale address cannot authorize a new identity)', () => {
    reg.register(CANON, ETH_ADDR)
    expect(reg.isModeAIdentity(CANON)).toBe(true)

    reg.reset() // logout / account switch

    expect(reg.ethAddressForPkHash(CANON)).toBeNull()
    expect(reg.isModeAIdentity(CANON)).toBe(false)
    // and looking up in any surface form is still cleared.
    expect(reg.ethAddressForPkHash(WITH_0X_UPPER)).toBeNull()
  })

  test('instances never cross bindings (module-scoped Map is now an instance field)', () => {
    const a = new ModeARegistry()
    const b = new ModeARegistry()
    a.register(CANON, ETH_ADDR)
    // b never saw the binding → held-key path, no cross-client leak.
    expect(b.ethAddressForPkHash(CANON)).toBeNull()
    expect(a.ethAddressForPkHash(CANON)).toBe(ETH_ADDR_LC)
  })
})
