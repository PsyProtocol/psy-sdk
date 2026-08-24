/**
 * Typed error taxonomy for @psy-protocol/evm-wallet.
 *
 * The ErrorContext union + copy table are ported verbatim from the mode-a
 * app's components/ErrorState.tsx (the classifier behavior is golden — see
 * classify.ts), extended with two module-level codes:
 *   - 'config_invalid'        — createPsyWallet input failed validation
 *   - 'network_not_deployed'  — a preset without a live deployment (mainnet)
 */

export type ErrorContext =
  | 'stuck_tx'
  | 'delayed_batch'
  | 'failed_relay'
  | 'network_congestion'
  | 'proof_failed'
  | 'sign_in_failed'
  | 'sign_in_rejected'
  | 'sign_in_busy'
  | 'generic'
  | 'config_invalid'
  | 'network_not_deployed'

/** Sign-in / registration contexts use login-appropriate copy and surface the
 *  RAW prover/registration error inline (never the payment-receipt copy). */
export const SIGN_IN_CONTEXTS: ReadonlySet<ErrorContext> = new Set<ErrorContext>([
  'sign_in_failed',
  'sign_in_rejected',
  'sign_in_busy',
])

export interface ErrorCopy {
  title: string
  summary: string
  assurance: string
  action: string
  severity: 'warning' | 'error' | 'info'
}

/**
 * Canonical error → user copy map (what happened / what happens next / what to
 * do). Ported from ErrorState.tsx so headless consumers and the app's UI speak
 * identically; 'MetaMask' generalized to 'wallet' (any EVM wallet signs now).
 */
export const errorInfo: Record<ErrorContext, ErrorCopy> = {
  stuck_tx: {
    title: 'Receipt still open',
    summary: 'The transaction is signed and waiting for inclusion.',
    assurance: 'Funds remain in the same protected path.',
    action: 'Leave it open or return later.',
    severity: 'warning',
  },
  delayed_batch: {
    title: 'Checkpoint delayed',
    summary: 'The batch proof is taking longer than usual.',
    assurance: 'The withdrawal stays queued until the batch root lands.',
    action: 'No action needed.',
    severity: 'info',
  },
  failed_relay: {
    title: 'Relayer unavailable',
    summary: 'The bridge could not reach the proof relayer.',
    assurance: 'If Ethereum accepted the deposit, the contract still holds it.',
    action: 'Retry after the relayer recovers.',
    severity: 'warning',
  },
  network_congestion: {
    title: 'Proof queue busy',
    summary: 'Psy is processing a larger proof batch.',
    assurance: 'Receipts are processed in order.',
    action: 'Do not resubmit.',
    severity: 'info',
  },
  proof_failed: {
    title: 'Proof failed',
    summary: 'The prover rejected this receipt.',
    assurance: 'No successful proof means no state was finalized.',
    action: 'Check the amount and retry.',
    severity: 'error',
  },
  sign_in_failed: {
    title: 'Sign-in failed',
    summary: 'We couldn’t set up your Psy account from this signature.',
    assurance: 'Nothing was charged and no account state changed.',
    action: 'See the details below, then try again.',
    severity: 'error',
  },
  sign_in_rejected: {
    title: 'Signature declined',
    summary: 'The wallet signature request was cancelled.',
    assurance: 'No account was created and nothing was charged.',
    action: 'Try again and approve the signature to continue.',
    severity: 'warning',
  },
  sign_in_busy: {
    title: 'Almost there',
    summary: 'The registration service is busy finishing your sign-in.',
    assurance: 'Your key may already be registered — funds are unaffected.',
    action: 'Wait a moment, then try again.',
    severity: 'info',
  },
  generic: {
    title: 'Receipt interrupted',
    summary: 'Something stopped the flow before completion.',
    assurance: 'If no chain receipt exists, no funds moved.',
    action: 'Try again in a moment.',
    severity: 'error',
  },
  config_invalid: {
    title: 'Configuration invalid',
    summary: 'createPsyWallet received an incomplete network definition.',
    assurance: 'Nothing was initialized.',
    action: 'Fix the reported field and re-create the client.',
    severity: 'error',
  },
  network_not_deployed: {
    title: 'Network not available',
    summary: 'This network preset has no live deployment.',
    assurance: 'Nothing was initialized.',
    action: 'Use the testnet preset or pass a custom network definition.',
    severity: 'error',
  },
}

/** Typed error thrown by the module. `code` drives UI copy via errorInfo. */
export class PsyWalletError extends Error {
  readonly code: ErrorContext
  readonly recoverable: boolean

  constructor(
    code: ErrorContext,
    message: string,
    opts?: { recoverable?: boolean; cause?: unknown },
  ) {
    super(message)
    this.name = 'PsyWalletError'
    this.code = code
    this.recoverable = opts?.recoverable ?? true
    if (opts?.cause !== undefined) {
      ;(this as { cause?: unknown }).cause = opts.cause
    }
  }
}
