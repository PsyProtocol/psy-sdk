/**
 * @psy-protocol/evm-wallet — EVM wallet integration for the Psy network.
 *
 * The connected EVM wallet IS the key: sign-in and every operation (deposit,
 * withdraw, transfer, private transfer, claim, UPS proof sessions) are
 * authorized by the wallet's `personal_sign` (EIP-191) — no held key, no seed
 * phrase. ZK proofs are generated client-side (Web Worker WASM prover from the
 * peer @psy-protocol/psy-sdk) and submitted to the Psy network.
 *
 * Headless core. React bindings live at '@psy-protocol/evm-wallet/react'; the
 * prover worker module at '@psy-protocol/evm-wallet/worker'.
 */

// ── config kernel ────────────────────────────────────────────────────────────
export {
  definePsyNetwork,
  type ChainConfig,
  type DeepPartial,
  type PsyChainConfig,
  type PsyNetworkDefinition,
  type PsyNetworkName,
  type PsyNetworkUrls,
  type PsyStorage,
  type PsyWalletConfig,
  type ResolvedNetwork,
} from './config/types'
export { resolveNetworkDefinition } from './config/resolve'
export { testnet } from './config/presets/testnet'
export { localhost } from './config/presets/localhost'

// ── errors ───────────────────────────────────────────────────────────────────
export {
  PsyWalletError,
  SIGN_IN_CONTEXTS,
  errorInfo,
  type ErrorContext,
  type ErrorCopy,
} from './errors/types'
export { classifyError, classifyLoginError } from './errors/classify'

// ── session (UPS) ─────────────────────────────────────────────────────────────
export {
  SessionController,
  type AuthVersion,
  type SessionPhase,
  type SessionState,
  type UnifiedSession,
  type UserIdPollProgress,
} from './session/controller'
export {
  AUTH_MESSAGE_VERSION,
  DEFAULT_AUTH_VERSION,
  EIP712_AUTH_VERSION,
  buildAuthMessage,
  deriveKeyMaterialFromSignature,
  deriveShieldAddressForUser,
  type DerivedPsyKeyMaterial,
} from './session/identity'
export { ModeARegistry } from './session/mode-a-registry'
export { ModeASubmitter, type ModeAResult } from './session/mode-a-submitter'

// ── psy rpc + types ───────────────────────────────────────────────────────────
export { RpcClient, realmIdFromUserId, type UserLeaf } from './psy/rpc'
export type { ClaimBatchItem, ContractCallArgs } from './psy/types'

// ── evm signer ────────────────────────────────────────────────────────────────
export { EvmSigner } from './evm/signer'

// ── runtime ───────────────────────────────────────────────────────────────────
export { createRuntime, type PsyWalletRuntime } from './runtime'
export { ProverEngine } from './prover/engine'

export const EVM_WALLET_VERSION = '0.1.0'
