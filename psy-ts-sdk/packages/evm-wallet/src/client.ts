/**
 * createPsyWallet — the module's public entry.
 *
 * The SDK owns exactly four things: the SIGNER (personal_sign → the SDK's
 * external-signature seam), the IDENTITY (derived from an EVM signature), the
 * SESSION (login/register/UPS lifecycle), and the PROVER (web Worker host).
 * Everything else — transfers, claims, inbox, activity, deposit, withdraw, and
 * all UI — stays in the consuming app. An app swaps ONLY its signing
 * implementation by pointing at this client; its ops call the exposed signer.
 *
 *   const wallet = createPsyWallet({ network: 'testnet', wagmiConfig, createWorker })
 *   await wallet.session.login()                     // any EVM wallet, personal_sign
 *   const { txHash } = await wallet.signer.execPublicContractCall(pkHash, calls, addr)
 */

import { resolveNetworkDefinition } from './config/resolve'
import type { PsyWalletConfig, ResolvedNetwork } from './config/types'
import { createRuntime, type PsyWalletRuntime } from './runtime'
import { ProverEngine } from './prover/engine'
import { EvmSigner } from './evm/signer'
import { RpcClient } from './psy/rpc'
import { ModeARegistry } from './session/mode-a-registry'
import { ModeASubmitter } from './session/mode-a-submitter'
import { SessionController } from './session/controller'

/** The assembled client. Holds the SDK's four owned concerns; the app reads
 *  `signer`/`session`/`identity`/`prover` and keeps its own ops + UI. */
export interface PsyWalletClient {
  /** UPS session: login/logout, phases, identity, account-switch handling. */
  readonly session: SessionController
  /** Mode-A transaction signer: the personal_sign implementation of the SDK's
   *  external-signature seam. The app's ops call this to submit under Mode-A. */
  readonly signer: ModeASubmitter
  /** pkHash→evmAddress registry: the Mode-A submit-path selector. */
  readonly registry: ModeARegistry
  /** Low-level personal_sign seam (connect + raw EIP-191 signatures). */
  readonly evmSigner: EvmSigner
  /** The prover pipeline (worker + FIFO gate); also drives held-key operators. */
  readonly prover: ProverEngine
  /** Coordinator/realm/prove-proxy JSON-RPC bound to this network. */
  readonly rpc: RpcClient
  /** The resolved network (config + derived URLs). */
  readonly network: ResolvedNetwork
  /** The underlying runtime (advanced consumers). */
  readonly runtime: PsyWalletRuntime
  /** Release the account-switch watcher + session listeners + prover worker. */
  dispose(): void
}

/**
 * Assemble a Psy wallet client from a network preset (or custom definition) and
 * the app's shared wagmi config. The SESSION, SIGNER, IDENTITY and PROVER come
 * from here; the app supplies everything else.
 */
export function createPsyWallet(config: PsyWalletConfig): PsyWalletClient {
  const network = resolveNetworkDefinition(config.network, config.overrides)
  const runtime = createRuntime({
    network,
    wagmiConfig: config.wagmiConfig,
    storage: config.storage,
    createWorker: config.createWorker,
  })
  const session = new SessionController(runtime)

  return {
    session,
    signer: session.submitter,
    registry: session.registry,
    evmSigner: session.evmSigner,
    prover: runtime.prover,
    rpc: session.rpc,
    network,
    runtime,
    dispose() {
      session.dispose()
      runtime.prover.terminate()
    },
  }
}
