import assert from 'node:assert/strict'
import { test } from 'node:test'
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'

const script = fileURLToPath(new URL('./gen-presets.mjs', import.meta.url))
const preset = new URL('../src/config/presets/testnet.ts', import.meta.url)
const client = () => ({ defaultNetwork: 'testnet', networks: { testnet: {
  magic: '0x1337CF514544CF69', prove_proxy_url: ['https://prover.example.org'],
  nostr_relay_url: 'wss://nostr.example.org',
} } })
const deployments = () => ({ l1_chains: [{
  chain_id: 11155111, chain_index: 0, rpc_url: 'https://rpc.example.org',
  contracts: Object.fromEntries(['Router', 'Bridge', 'StateManager', 'ERC20Gateway', 'WETH9', 'USDTToken', 'PsyToken']
    .map(key => [key, `0x${'1'.repeat(40)}`])),
}] })

for (const [name, change] of [
  ['wrong default network', (c, _d) => { c.defaultNetwork = 'localhost' }],
  ['duplicate Sepolia', (_c, d) => { d.l1_chains.push(d.l1_chains[0]) }],
  ['zero bridge', (_c, d) => { d.l1_chains[0].contracts.Bridge = `0x${'0'.repeat(40)}` }],
  ['non-TLS prover', (c, _d) => { c.networks.testnet.prove_proxy_url = ['http://prover.example.org'] }],
  ['numeric host', (_c, d) => { d.l1_chains[0].rpc_url = 'https://127.0.0.1' }],
  ['URL credentials', (_c, d) => { d.l1_chains[0].rpc_url = 'https://user:placeholder@rpc.example.org' }],
]) {
  test(`public preset rejects ${name} without replacing the preset`, () => {
    const before = readFileSync(preset)
    const dir = mkdtempSync(join(tmpdir(), 'psy-preset-test-'))
    try {
      const c = client(), d = deployments()
      change(c, d)
      writeFileSync(join(dir, 'client.json'), JSON.stringify(c))
      writeFileSync(join(dir, 'deployments.json'), JSON.stringify(d))
      const run = spawnSync(process.execPath, [script, '--public-config', join(dir, 'client.json'), join(dir, 'deployments.json')], { encoding: 'utf8' })
      assert.equal(run.status, 1)
      assert.match(run.stderr, /AssertionError/)
      assert.deepEqual(readFileSync(preset), before)
    } finally {
      rmSync(dir, { recursive: true })
    }
  })
}
