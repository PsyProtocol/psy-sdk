import { describe, expect, test } from '@jest/globals'
import { resolveNetworkDefinition, deepMerge, firstConfiguredUrl } from '../src/config/resolve'
import { definePsyNetwork } from '../src/config/types'
import { testnet } from '../src/config/presets/testnet'
import { localhost } from '../src/config/presets/localhost'
import { PsyWalletError } from '../src/errors/types'

describe('config: presets', () => {
  test('testnet uses the retained protocol and public prover', () => {
    const net = resolveNetworkDefinition('testnet')
    expect(BigInt(net.psy.magic)).toBe(0x1337cf514544cf69n)
    expect(net.urls.proveProxy).toBe('https://prove-stg.psy-protocol.xyz')
    expect(net.urls.nostrRelays).toEqual(['wss://nostr-stg.psy-protocol.xyz/'])
    expect(net.urls.l1Rpc).toEqual(['https://rpc-eth-stg.psy-protocol.xyz'])
  })

  test('testnet targets the fresh Sepolia deployment', () => {
    const net = resolveNetworkDefinition('testnet')
    expect(net.l1.chain.bridgeAddress).toBe('0xb133107eEA4D9B255e9721EE2cACaB56747eEeBd')
    expect(net.l1.chain.routerAddress).toBe('0x947528f677CF514B35393fE5FBb2A31b444C11A8')
    expect(net.l1.chain.stateManagerAddress).toBe('0x4eC67bA22A3F7a9Af1f19F480e5Df33bFD7B06F1')
  })

  test('testnet preset resolves with derived urls', () => {
    const net = resolveNetworkDefinition('testnet')
    expect(net.name).toBe('testnet')
    expect(net.l1.chainId).toBe(11155111)
    expect(net.urls.coordinator).toMatch(/^https?:\/\//)
    expect(net.urls.realm(0)).toMatch(/^https?:\/\//)
    expect(net.l1.chain.routerAddress).toMatch(/^0x[0-9a-fA-F]{40}$/)
    expect(net.l1.chain.bridgeAddress).toMatch(/^0x[0-9a-fA-F]{40}$/)
  })

  test('localhost preset resolves (chainId 31337)', () => {
    const net = resolveNetworkDefinition('localhost')
    expect(net.l1.chainId).toBe(31337)
    expect(net.urls.realm(1)).toBeTruthy()
  })

  test('mainnet preset throws network_not_deployed', () => {
    try {
      resolveNetworkDefinition('mainnet')
      throw new Error('should have thrown')
    } catch (e) {
      expect(e).toBeInstanceOf(PsyWalletError)
      expect((e as PsyWalletError).code).toBe('network_not_deployed')
      expect((e as PsyWalletError).recoverable).toBe(false)
    }
  })
})

describe('config: overrides', () => {
  test('deep-partial overrides merge over the preset without mutating it', () => {
    const proxyBefore = testnet.psy.prove_proxy_url
    const net = resolveNetworkDefinition('testnet', {
      psy: { prove_proxy_url: ['http://127.0.0.1:9999'] },
      l1: { chain: { routerAddress: '0x' + '11'.repeat(20) } },
    })
    expect(net.urls.proveProxy).toBe('http://127.0.0.1:9999')
    expect(net.l1.chain.routerAddress).toBe('0x' + '11'.repeat(20))
    // untouched fields survive
    expect(net.l1.chain.bridgeAddress).toBe(testnet.l1.chain.bridgeAddress)
    expect(net.l1.chainId).toBe(11155111)
    // the preset object itself is not mutated
    expect(testnet.psy.prove_proxy_url).toBe(proxyBefore)
  })

  test('arrays replace, objects merge', () => {
    const merged = deepMerge(
      { a: { x: 1, y: 2 }, list: [1, 2, 3] },
      { a: { y: 9 }, list: [7] },
    )
    expect(merged).toEqual({ a: { x: 1, y: 9 }, list: [7] })
  })
})

describe('config: validation', () => {
  test('missing coordinator throws config_invalid', () => {
    const bad = definePsyNetwork({
      name: 'custom',
      psy: { ...localhost.psy, coordinator_configs: [] },
      l1: localhost.l1,
    })
    try {
      resolveNetworkDefinition(bad)
      throw new Error('should have thrown')
    } catch (e) {
      expect((e as PsyWalletError).code).toBe('config_invalid')
    }
  })

  test('custom definition via definePsyNetwork resolves', () => {
    const custom = definePsyNetwork({
      name: 'my-devnet',
      psy: localhost.psy,
      l1: localhost.l1,
    })
    const net = resolveNetworkDefinition(custom, undefined)
    expect(net.name).toBe('my-devnet')
  })
})

describe('firstConfiguredUrl', () => {
  test('string, array, empty', () => {
    expect(firstConfiguredUrl('http://a')).toBe('http://a')
    expect(firstConfiguredUrl(['', ' ', 'http://b'])).toBe('http://b')
    expect(firstConfiguredUrl(undefined)).toBe('')
    expect(firstConfiguredUrl([])).toBe('')
  })
})
