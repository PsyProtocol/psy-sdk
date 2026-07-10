import { describe, expect, test } from '@jest/globals'
import { classifyError, classifyLoginError } from '../src/errors/classify'
import { SIGN_IN_CONTEXTS, errorInfo } from '../src/errors/types'

/** Behavior table ported from the app's ErrorState.tsx — these matchers are
 *  load-bearing (real error strings observed in production flows). */
describe('classifyError (payment contexts)', () => {
  test.each([
    ['request timeout after 30s', 'stuck_tx'],
    ['tx still pending', 'stuck_tx'],
    ['checkpoint batch not yet finalized', 'delayed_batch'],
    ['proof queue backlog', 'delayed_batch'],
    ['relayer unreachable', 'failed_relay'],
    ['bridge endpoint 502', 'failed_relay'],
    ['network congestion detected', 'network_congestion'],
    ['prover overloaded', 'network_congestion'],
    ['proof failed at call_index=0', 'proof_failed'],
    ['circuit rejected witness', 'proof_failed'],
    ['verify error in gate 3', 'proof_failed'],
    ['something completely else', 'generic'],
  ])('%s → %s', (msg, expected) => {
    expect(classifyError(msg)).toBe(expected)
  })
})

describe('classifyLoginError (sign-in contexts)', () => {
  test.each([
    ['MetaMask Tx Signature: User denied transaction signature. code 4001', 'sign_in_rejected'],
    ['user rejected the request', 'sign_in_rejected'],
    ['User cancelled', 'sign_in_rejected'],
    ['registration service busy, retry shortly', 'sign_in_busy'],
    ['network_congestion', 'sign_in_busy'],
    ['get_user_ids_for_public_key: no user ids found', 'sign_in_failed'],
    ['VirtualTarget { index: 516 } mismatch', 'sign_in_failed'],
  ])('%s → %s', (msg, expected) => {
    expect(classifyLoginError(msg)).toBe(expected)
  })

  test('login classifications are all SIGN_IN_CONTEXTS members', () => {
    for (const msg of ['4001', 'busy', 'anything']) {
      expect(SIGN_IN_CONTEXTS.has(classifyLoginError(msg))).toBe(true)
    }
  })
})

describe('errorInfo copy table', () => {
  test('every ErrorContext has complete copy', () => {
    for (const [code, copy] of Object.entries(errorInfo)) {
      expect(copy.title).toBeTruthy()
      expect(copy.summary).toBeTruthy()
      expect(copy.assurance).toBeTruthy()
      expect(copy.action).toBeTruthy()
      expect(['warning', 'error', 'info']).toContain(copy.severity)
      expect(code).toBeTruthy()
    }
  })
})
