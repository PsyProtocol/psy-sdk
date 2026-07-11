import { describe, expect, test } from '@jest/globals'
import {
  pollUserIdAcrossCheckpoints,
  POLL_BUSY_MESSAGE,
  type UserIdPollDeps,
} from '../src/session/userIdPoll'
import { deriveKeyMaterialFromSignature } from '../src/session/identity'
import { realmIdFromUserId } from '../src/psy/rpc'
import { ModeARegistry } from '../src/session/mode-a-registry'

/** A deterministic 65-byte-ish signature hex for derivation tests (the input is
 *  just hashed, so any stable hex works). */
const SIG_A =
  '0x' + 'ab'.repeat(65)
const SIG_B =
  '0x' + 'cd'.repeat(65)
const ADDR = '0x1234567890abcdef1234567890abcdef12345678'

describe('userIdPoll: timing seams', () => {
  function fakeDeps(over: Partial<UserIdPollDeps>): UserIdPollDeps {
    return {
      resolve: async () => null,
      getCheckpoint: async () => 0,
      sleep: async () => {},
      now: () => 0,
      ...over,
    }
  }

  test('attempt-0 resolves immediately with no sleep (returning user)', async () => {
    let slept = 0
    const id = await pollUserIdAcrossCheckpoints('0xpk', undefined, fakeDeps({
      resolve: async () => '917504',
      sleep: async () => {
        slept++
      },
    }))
    expect(id).toBe('917504')
    expect(slept).toBe(0)
  })

  test('resolves after N polls once the id appears', async () => {
    let calls = 0
    const id = await pollUserIdAcrossCheckpoints('0xpk', undefined, fakeDeps({
      resolve: async () => (++calls >= 3 ? '42' : null),
    }))
    expect(id).toBe('42')
  })

  test('wall-clock cap exhaustion throws the calm busy message', async () => {
    let t = 0
    await expect(
      pollUserIdAcrossCheckpoints('0xpk', undefined, fakeDeps({
        resolve: async () => null,
        // advance wall-clock past the 900s cap on each poll
        now: () => (t += 1_000_000),
        getCheckpoint: async () => 0,
      })),
    ).rejects.toThrow(POLL_BUSY_MESSAGE)
  })

  test('checkpoint read failures degrade to wall-clock (never hang)', async () => {
    let t = 0
    await expect(
      pollUserIdAcrossCheckpoints('0xpk', undefined, fakeDeps({
        resolve: async () => null,
        getCheckpoint: async () => {
          throw new Error('rpc down')
        },
        now: () => (t += 1_000_000),
      })),
    ).rejects.toThrow(POLL_BUSY_MESSAGE)
  })
})

describe('identity: deterministic derivation', () => {
  test('same signature → identical key material (cross-device recovery)', async () => {
    const a1 = await deriveKeyMaterialFromSignature(ADDR, SIG_A)
    const a2 = await deriveKeyMaterialFromSignature(ADDR, SIG_A)
    expect(a2).toEqual(a1)
    expect(a1.psyPrivateKeyHex).toMatch(/^0x[0-9a-f]{64}$/)
    expect(a1.evmAddress).toBe(ADDR.toLowerCase())
  })

  test('different signatures → different Psy keys', async () => {
    const a = await deriveKeyMaterialFromSignature(ADDR, SIG_A)
    const b = await deriveKeyMaterialFromSignature(ADDR, SIG_B)
    expect(b.psyPrivateKeyHex).not.toBe(a.psyPrivateKeyHex)
    expect(b.random0).not.toBe(a.random0)
  })

  test('r0/r1 reduced into the Goldilocks field', async () => {
    const GOLDILOCKS = 0xffffffff00000001n
    const a = await deriveKeyMaterialFromSignature(ADDR, SIG_A)
    expect(BigInt(a.random0) < GOLDILOCKS).toBe(true)
    expect(BigInt(a.random1) < GOLDILOCKS).toBe(true)
  })
})

describe('realmIdFromUserId', () => {
  test('floor(userId / usersPerRealm)', () => {
    expect(realmIdFromUserId(0n, 1048576)).toBe(0)
    expect(realmIdFromUserId(1048575n, 1048576)).toBe(0)
    expect(realmIdFromUserId(1048576n, 1048576)).toBe(1)
    expect(realmIdFromUserId(917504n, 1048576)).toBe(0)
  })
  test('zero users_per_realm degrades to realm 0', () => {
    expect(realmIdFromUserId(999n, 0)).toBe(0)
  })
})

describe('ModeARegistry: instance isolation', () => {
  test('register/lookup/reset, 0x-insensitive', () => {
    const reg = new ModeARegistry()
    reg.register('0xABCD', '0xEEEE')
    expect(reg.isModeAIdentity('abcd')).toBe(true)
    expect(reg.ethAddressForPkHash('0xabcd')).toBe('0xeeee')
    expect(reg.isModeAIdentity('0xffff')).toBe(false)
    reg.reset()
    expect(reg.isModeAIdentity('abcd')).toBe(false)
  })

  test('two registries never share bindings', () => {
    const a = new ModeARegistry()
    const b = new ModeARegistry()
    a.register('0x01', '0xaa')
    expect(b.isModeAIdentity('0x01')).toBe(false)
  })
})
