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

const here = dirname(fileURLToPath(import.meta.url))
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
