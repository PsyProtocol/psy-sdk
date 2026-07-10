/**
 * Configuration types for @psy-protocol/evm-wallet.
 *
 * Everything the mode-a app previously read from `import.meta.env.VITE_*` and
 * the `@chain-config` / `@protocol-config` / `@deployments` build-time aliases
 * is INJECTED through `PsyWalletConfig` instead — the package has zero
 * environment or build-alias coupling. Presets (testnet/localhost) ship as
 * data snapshots (see ./presets, regenerable via scripts/gen-presets.mjs); a
 * consumer with live values passes them via `overrides` or a full custom
 * `definePsyNetwork(...)`.
 */

import type { Config as WagmiConfig } from 'wagmi'

export type PsyNetworkName = 'mainnet' | 'testnet' | 'localhost'

/**
 * Raw Psy chain config — the `networks.<name>` object of the protocol
 * config.json, passed to the WASM prover verbatim (PsyJSON.stringify of this
 * object constructs the prover). Structurally typed for the fields this module
 * reads; all other fields are preserved untouched.
 */
export interface PsyChainConfig {
  magic: string
  users_per_realm: number
  global_user_tree_height: number
  realm_user_tree_height: number
  group_realm_height: number
  realm_configs: Array<{ rpc_url: string[]; [key: string]: unknown }>
  coordinator_configs: Array<{ rpc_url: string[]; [key: string]: unknown }>
  prove_proxy_url?: string[] | string
  api_services_url?: string | string[]
  indexer_graphql_url?: string | string[]
  explorer_url?: string | string[]
  nostr_relay_urls?: string[]
  native_currency?: string
  native_currency_decimal?: number
  native_currency_name?: string
  native_currency_symbol?: string
  fees?: Record<string, number | string>
  l1_rpc_urls?: string[]
  [key: string]: unknown
}

/**
 * L1 (EVM) chain description incl. the bridge contract addresses.
 * Ported from the app's `src/bridge/types/index.ts` ChainConfig — the shape the
 * deposit/withdraw engine consumes.
 */
export interface ChainConfig {
  chainId: number
  name: string
  shortName: string
  /** PSY internal chain index (0-255). */
  psyIndex: number
  nativeCurrency: { name: string; symbol: string; decimals: number }
  explorerUrl: string
  rpcUrls: string[]
  routerAddress: string
  bridgeAddress: string
  stateManagerAddress: string
  erc20GatewayAddress: string
  wethAddress: string
  stateManager?: string
  erc20Gateway?: string
  mockUSDT?: string
  psyToken?: string
  deployed?: boolean
}

/** A complete network definition: the Psy side + the L1 side. */
export interface PsyNetworkDefinition {
  /** Display/name key, e.g. 'testnet', 'localhost', or a custom name. */
  name: string
  /** Raw Psy chain config (prover construction + URL derivation source). */
  psy: PsyChainConfig
  /** The L1 EVM chain the bridge deposits from / withdraws to. */
  l1: {
    chainId: number
    chain: ChainConfig
  }
}

/** Service URLs derived from a PsyNetworkDefinition (see resolve.ts). */
export interface PsyNetworkUrls {
  coordinator: string
  realm: (realmId: number | bigint) => string
  proveProxy: string
  psyServices: string
  indexer: string
  explorer: string
  nostrRelays: readonly string[]
  l1Rpc: readonly string[]
}

/** A resolved, validated network: definition + derived URLs. */
export interface ResolvedNetwork extends PsyNetworkDefinition {
  urls: PsyNetworkUrls
}

/** Minimal storage seam (defaults to window.localStorage; injectable for tests/SSR). */
export interface PsyStorage {
  get(key: string): string | null
  set(key: string, value: string): void
  remove(key: string): void
}

export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K]
    : T[K] extends (...args: never[]) => unknown
      ? T[K]
      : T[K] extends object
        ? DeepPartial<T[K]>
        : T[K]
}

/** Input to createPsyWallet(). */
export interface PsyWalletConfig {
  /** Preset name or a full custom definition (see definePsyNetwork). */
  network: PsyNetworkName | PsyNetworkDefinition
  /** Deep-partial override merged over the preset (URLs, fees, addresses…). */
  overrides?: DeepPartial<PsyNetworkDefinition>
  /**
   * The app's shared wagmi config — REQUIRED. Every wallet connect and every
   * personal_sign (login AND per-transaction) routes through it, so the app
   * and this module always agree on the connected account.
   */
  wagmiConfig: WagmiConfig
  /**
   * Prover Worker factory. RECOMMENDED: point it at a 1-line app-local file
   * that does `import '@psy-protocol/evm-wallet/worker'` so YOUR bundler
   * resolves the worker URL (see README). If omitted, the package attempts to
   * spawn its own dist worker, which requires bundler cooperation
   * (e.g. Vite `optimizeDeps.exclude`).
   */
  createWorker?: () => Worker
  /** Storage seam; defaults to window.localStorage when available. */
  storage?: PsyStorage
}

/** Identity helper for typed custom network definitions. */
export function definePsyNetwork(def: PsyNetworkDefinition): PsyNetworkDefinition {
  return def
}
