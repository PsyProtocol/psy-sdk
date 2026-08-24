/**
 * Error classifiers — ported VERBATIM from the mode-a app's
 * components/ErrorState.tsx. The string-matching behavior is load-bearing
 * (drives which recovery copy users see); do not "improve" the heuristics
 * without cross-checking the app's error strings.
 */

import type { ErrorContext } from './types'

/** Detect error type from a raw error message (PAYMENT/receipt contexts). */
export function classifyError(error: string): ErrorContext {
  const e = error.toLowerCase()
  if (e.includes('timeout') || e.includes('pending') || e.includes('not confirmed')) return 'stuck_tx'
  if (e.includes('batch') || e.includes('finalize') || e.includes('queue')) return 'delayed_batch'
  if (e.includes('relay') || e.includes('relayer') || e.includes('bridge')) return 'failed_relay'
  if (e.includes('congestion') || e.includes('overloaded') || e.includes('capacity')) return 'network_congestion'
  if (e.includes('proof') || e.includes('circuit') || e.includes('verify')) return 'proof_failed'
  return 'generic'
}

/**
 * Classify a SIGN-IN / REGISTRATION failure. Distinct from classifyError so a
 * login failure NEVER renders the payment "receipt" copy ("check the amount").
 * A failed registration proof (incl. the circuit/VirtualTarget mismatch class)
 * maps to 'sign_in_failed', which surfaces the raw prover error inline.
 */
export function classifyLoginError(error: string): ErrorContext {
  const e = error.toLowerCase()
  // EIP-1193 user rejection (code 4001) or an explicit decline.
  if (
    e.includes('4001') ||
    e.includes('user rejected') ||
    e.includes('user denied') ||
    e.includes('rejected the request') ||
    e.includes('cancel')
  ) {
    return 'sign_in_rejected'
  }
  // Registration indexed but not yet checkpointed — transient, just retry.
  if (e.includes('network_congestion') || e.includes('busy') || e.includes('congestion')) {
    return 'sign_in_busy'
  }
  // Everything else (incl. prover/circuit/proof rejections) is a real sign-in
  // failure; the raw message is shown inline so the true cause is visible.
  return 'sign_in_failed'
}
