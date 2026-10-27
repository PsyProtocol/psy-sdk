#!/usr/bin/env node
/**
 * Regenerate src/config/presets/{testnet,localhost}.ts from the live protocol
 * sources in a parth-generic-v1 checkout:
 *   - <parth>/client_prover/config.json          (networks.<name> — the Psy chain config)
 *   - <parth>/psy-contracts/deployments/<net>/deployed-contracts.json (L1 addresses)
 *
 * Usage: node scripts/gen-presets.mjs [/path/to/parth-generic-v1]
 * Default parth path assumes the sibling checkout layout used in this repo's
 * development environment.
 */
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createHash } from 'node:crypto'
import assert from 'node:assert/strict'

const here = dirname(fileURLToPath(import.meta.url))

// Public builds consume reviewed client and deployment exports, never node topology.
if (process.argv[2] === '--public-config') {
  assert.equal(process.argv.length, 5, 'Usage: --public-config client.json deployments.json')
  const { PsyJSON } = await import('@psy-protocol/psy-sdk')
  const clientBytes = readFileSync(resolve(process.argv[3]), 'utf8')
  const deploymentBytes = readFileSync(resolve(process.argv[4]), 'utf8')
  const client = PsyJSON.parse(clientBytes)
  const deployments = JSON.parse(deploymentBytes)
  assert.equal(client.defaultNetwork, 'testnet')
  assert(client.networks?.testnet, 'Missing testnet client config')
  const rows = deployments.l1_chains.filter(row => row.chain_id === 11155111)
  assert.equal(rows.length, 1, 'Expected exactly one Sepolia deployment')
  const row = rows[0]
  assert.equal(row.chain_index, 0)
  const core = row.contracts
  for (const key of ['Router', 'Bridge', 'StateManager', 'ERC20Gateway', 'WETH9', 'USDTToken', 'PsyToken']) {
    assert(/^0x[0-9a-fA-F]{40}$/.test(core[key]) && !/^0x0{40}$/.test(core[key]), `Invalid ${key}`)
  }
  const psy = {
    ...client.networks.testnet,
    nostr_relay_urls: client.networks.testnet.nostr_relay_urls ?? [client.networks.testnet.nostr_relay_url],
    // This preset selects Sepolia; BSC/Base RPCs cannot be fallback endpoints for it.
    l1_rpc_urls: [row.rpc_url],
  }
  const definition = {
    name: 'testnet', psy,
    l1: {
      chainId: row.chain_id,
      chain: {
        chainId: row.chain_id, name: row.name, shortName: row.network, psyIndex: row.chain_index,
        nativeCurrency: row.native_currency, explorerUrl: row.explorer_url, rpcUrls: [row.rpc_url],
        routerAddress: core.Router, bridgeAddress: core.Bridge, stateManagerAddress: core.StateManager,
        erc20GatewayAddress: core.ERC20Gateway, wethAddress: core.WETH9,
        stateManager: core.StateManager, erc20Gateway: core.ERC20Gateway,
        mockUSDT: core.USDTToken, psyToken: core.PsyToken, deployed: true,
      },
    },
  }
  function checkUrls(value) {
    if (Array.isArray(value)) value.forEach(checkUrls)
    else if (value && typeof value === 'object') Object.values(value).forEach(checkUrls)
    else if (typeof value === 'string' && /^(?:https?|wss?):\/\//.test(value)) {
      const url = new URL(value)
      assert(['https:', 'wss:'].includes(url.protocol), 'Public preset needs TLS')
      assert(!url.username && !url.password, 'Credential-bearing URL')
      assert(/^[a-z0-9.-]+\.[a-z]{2,}$/i.test(url.hostname), 'Public DNS name required')
      assert(!url.hostname.endsWith('.internal') && !url.hostname.endsWith('.local'), 'Internal hostname')
    }
  }
  checkUrls(definition)
  assert(psy.nostr_relay_urls.every(url => typeof url === 'string' && url.startsWith('wss://')))
  const digest = bytes => createHash('sha256').update(bytes).digest('hex')
  const body = `/**
 * Generated from public client SHA256 ${digest(clientBytes)}
 * and public deployment SHA256 ${digest(deploymentBytes)}.
 * Regenerate: node scripts/gen-presets.mjs --public-config client.json deployments.json
 */
import { PsyJSON } from '@psy-protocol/psy-sdk'
import type { PsyNetworkDefinition } from '../types'

export const testnet = PsyJSON.parse(${JSON.stringify(PsyJSON.stringify(definition))}) as PsyNetworkDefinition
`
  const out = join(here, '../src/config/presets/testnet.ts')
  writeFileSync(out, body)
  console.log(`wrote testnet preset (client ${digest(clientBytes)}, deployments ${digest(deploymentBytes)})`)
  process.exit(0)
}
const parth = resolve(
  process.argv[2] ?? join(here, '../../../../..', 'parth-generic-v1'),
)

const config = JSON.parse(
  readFileSync(join(parth, 'client_prover/config.json'), 'utf8'),
)

// preset name -> { config.json network key, deployments dir }
const TARGETS = [
  { preset: 'testnet', network: 'sepolia' },
  { preset: 'localhost', network: 'localhost' },
]

function loadDeployment(network) {
  return JSON.parse(
    readFileSync(
      join(parth, 'psy-contracts/deployments', network, 'deployed-contracts.json'),
      'utf8',
    ),
  )
}

function first(value) {
  if (Array.isArray(value)) return value.find((v) => typeof v === 'string' && v.trim()) ?? ''
  return typeof value === 'string' ? value : ''
}

for (const { preset, network } of TARGETS) {
  const psy = config.networks[network]
  if (!psy) throw new Error(`config.json has no networks.${network}`)
  const dep = loadDeployment(network)
  const core = dep.core ?? {}
  const tokens = dep.protocol?.tokens ?? {}
  const chainId = Number(dep.chainId)

  const chain = {
    chainId,
    name: network === 'sepolia' ? 'Sepolia' : 'Localhost',
    shortName: network === 'sepolia' ? 'sepolia' : 'localhost',
    psyIndex: 0,
    nativeCurrency: { name: 'Ether', symbol: 'ETH', decimals: 18 },
    explorerUrl:
      network === 'sepolia' ? 'https://sepolia.etherscan.io' : '',
    rpcUrls: psy.l1_rpc_urls ?? [],
    routerAddress: core.Router ?? '',
    bridgeAddress: core.Bridge ?? '',
    stateManagerAddress: core.StateManager ?? '',
    erc20GatewayAddress: core.ERC20Gateway ?? '',
    wethAddress: core.WETH9 ?? core.WETH ?? '',
    stateManager: core.StateManager ?? '',
    erc20Gateway: core.ERC20Gateway ?? '',
    mockUSDT: tokens.USDT?.l1Address ?? '',
    psyToken: tokens.PSY?.l1Address ?? '',
    deployed: Boolean(core.Router && core.Bridge),
  }

  const body = `/**
 * GENERATED preset — do not edit by hand.
 * Source: parth-generic-v1 client_prover/config.json networks.${network}
 *       + psy-contracts/deployments/${network}/deployed-contracts.json
 *       (generatedAt: ${dep.generatedAt ?? 'unknown'})
 * Regenerate: node scripts/gen-presets.mjs [/path/to/parth-generic-v1]
 * Live consumers can always pass fresher values via createPsyWallet overrides.
 */
import type { PsyNetworkDefinition } from '../types'

export const ${preset}: PsyNetworkDefinition = ${JSON.stringify(
    { name: preset, psy, l1: { chainId, chain } },
    null,
    2,
  )} as unknown as PsyNetworkDefinition
`
  const out = join(here, '../src/config/presets', `${preset}.ts`)
  mkdirSync(dirname(out), { recursive: true })
  writeFileSync(out, body)
  console.log(`wrote ${out} (chainId ${chainId}, router ${chain.routerAddress.slice(0, 10)}…)`)
}
