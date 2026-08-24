/**
 * Network resolution: preset-or-custom + deep-partial overrides → a validated
 * ResolvedNetwork with derived service URLs.
 *
 * URL derivation mirrors the app's services/chainConfig.ts exactly:
 *   coordinator = psy.coordinator_configs[0].rpc_url[0]
 *   realm(id)   = psy.realm_configs[id].rpc_url[0] (fallback realm 0)
 *   proveProxy  = first(psy.prove_proxy_url)
 *   psyServices = first(psy.api_services_url)
 *   indexer     = first(psy.indexer_graphql_url)
 *   explorer    = first(psy.explorer_url)
 *   nostrRelays = psy.nostr_relay_urls
 *   l1Rpc       = psy.l1_rpc_urls (fallback l1.chain.rpcUrls)
 */

import type {
  DeepPartial,
  PsyNetworkDefinition,
  PsyNetworkName,
  ResolvedNetwork,
} from './types'
import { PsyWalletError } from '../errors/types'
import { testnet } from './presets/testnet'
import { localhost } from './presets/localhost'

/** First URL out of a string-or-array config field. */
export function firstConfiguredUrl(value: unknown): string {
  if (Array.isArray(value)) {
    const first = value.find((v) => typeof v === 'string' && v.trim())
    return typeof first === 'string' ? first.trim() : ''
  }
  return typeof value === 'string' ? value.trim() : ''
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

/** Deep merge: arrays and primitives replace; plain objects merge. */
export function deepMerge<T>(base: T, override: DeepPartial<T> | undefined): T {
  if (override === undefined) return base
  if (!isPlainObject(base) || !isPlainObject(override)) {
    return (override as T) ?? base
  }
  const out: Record<string, unknown> = { ...base }
  for (const [key, value] of Object.entries(override)) {
    if (value === undefined) continue
    const baseValue = (base as Record<string, unknown>)[key]
    out[key] =
      isPlainObject(baseValue) && isPlainObject(value)
        ? deepMerge(baseValue, value as DeepPartial<typeof baseValue>)
        : value
  }
  return out as T
}

function presetFor(name: PsyNetworkName): PsyNetworkDefinition {
  switch (name) {
    case 'testnet':
      return testnet
    case 'localhost':
      return localhost
    case 'mainnet':
      // Placeholder until the mainnet deployment exists. Consumers can still
      // target a mainnet-like chain today via definePsyNetwork(...) with their
      // own values.
      throw new PsyWalletError(
        'network_not_deployed',
        "The 'mainnet' preset is not available yet — Psy mainnet contracts are not deployed. " +
          'Pass a custom definePsyNetwork({...}) definition, or use the testnet preset.',
        { recoverable: false },
      )
  }
}

/** Resolve + validate the network definition used by createPsyWallet. */
export function resolveNetworkDefinition(
  network: PsyNetworkName | PsyNetworkDefinition,
  overrides?: DeepPartial<PsyNetworkDefinition>,
): ResolvedNetwork {
  const base = typeof network === 'string' ? presetFor(network) : network
  const def = deepMerge(base, overrides)

  const coordinator = def.psy.coordinator_configs?.[0]?.rpc_url?.[0] ?? ''
  if (!coordinator) {
    throw new PsyWalletError(
      'config_invalid',
      `Network '${def.name}': psy.coordinator_configs[0].rpc_url[0] is required.`,
      { recoverable: false },
    )
  }
  if (!def.psy.realm_configs?.length) {
    throw new PsyWalletError(
      'config_invalid',
      `Network '${def.name}': psy.realm_configs must have at least one realm.`,
      { recoverable: false },
    )
  }
  if (!def.l1?.chainId) {
    throw new PsyWalletError(
      'config_invalid',
      `Network '${def.name}': l1.chainId is required.`,
      { recoverable: false },
    )
  }

  const urls = Object.freeze({
    coordinator,
    realm: (realmId: number | bigint): string => {
      const id = Number(realmId)
      return (
        def.psy.realm_configs[id]?.rpc_url?.[0] ??
        def.psy.realm_configs[0]?.rpc_url?.[0] ??
        ''
      )
    },
    proveProxy: firstConfiguredUrl(def.psy.prove_proxy_url),
    psyServices: firstConfiguredUrl(def.psy.api_services_url),
    indexer: firstConfiguredUrl(def.psy.indexer_graphql_url),
    explorer: firstConfiguredUrl(def.psy.explorer_url),
    nostrRelays: Object.freeze([
      ...(def.psy.nostr_relay_urls ?? []),
    ]) as readonly string[],
    l1Rpc: Object.freeze([
      ...(def.psy.l1_rpc_urls?.length ? def.psy.l1_rpc_urls : def.l1.chain.rpcUrls ?? []),
    ]) as readonly string[],
  })

  return { ...def, urls }
}
