/**
 * EVM-wallet signing seam — ported from the mode-a app's unified/metamask.ts.
 *
 * Every connect + signature routes through wagmi (`wagmi/actions` against the
 * client's shared `wagmiConfig`), so ANY EVM wallet RainbowKit can connect —
 * MetaMask, WalletConnect, Coinbase, OKX, … — drives the wallet. The connect
 * UI lives in React (RainbowKit's modal); these helpers only READ the connected
 * account and REQUEST signatures, so they stay callable from headless code.
 *
 * The ONLY change from the app version: `wagmiConfig` is an instance field
 * (injected from the runtime) instead of a module import — so two clients never
 * cross wires. Signing behavior is byte-identical.
 *
 * SIGNING IS `personal_sign` (EIP-191) ONLY — the login auth message and every
 * per-tx tx-hash signature. Raw `eth_sign` is gone (many wallets disable it).
 * `ethPersonalSign` uses `signMessage({ message: { raw } })`, byte-identical to
 * the legacy `personal_sign('0x' + sighash)`, so the on-chain
 * `EthPersonalSecp256K1` circuit verifies with NO change.
 */

import { verifyMessage, verifyTypedData, type TypedDataDomain } from 'ethers'
import type { Config as WagmiConfig } from 'wagmi'
import { getAccount, signMessage, signTypedData, watchAccount } from 'wagmi/actions'

type Hex = `0x${string}`

/** A single EIP-712 typed-data field. */
type Eip712Field = { name: string; type: string }

/** viem throws `UserRejectedRequestError` (EIP-1193 code 4001) on cancel. */
function isUserRejection(e: unknown): boolean {
  const err = e as { code?: number; name?: string; message?: string }
  return (
    err?.code === 4001 ||
    err?.name === 'UserRejectedRequestError' ||
    /user rejected|user denied|rejected the request/i.test(err?.message ?? '')
  )
}

const REJECT_MESSAGE =
  'You rejected signing. Approve the signature in your wallet to continue.'

export class EvmSigner {
  constructor(private readonly wagmiConfig: WagmiConfig) {}

  /**
   * True when an EVM wallet can be used. With RainbowKit the multi-wallet picker
   * is always available, so this is effectively always true; kept for the (few)
   * call sites that gate UI on wallet availability.
   */
  isAvailable(): boolean {
    return true
  }

  /** The address of the currently-connected EVM wallet, or null. */
  getConnectedAccount(): string | null {
    return getAccount(this.wagmiConfig).address ?? null
  }

  /**
   * Return the connected EVM address. The connect handshake happens in React via
   * RainbowKit BEFORE login runs, so by the time this is called a wallet is
   * expected to be connected; if not, surface an actionable error.
   */
  async connect(): Promise<string> {
    const address = getAccount(this.wagmiConfig).address
    if (!address) {
      throw new Error('Connect an EVM wallet first, then sign in.')
    }
    return address
  }

  /**
   * `personal_sign` a UTF-8 message (the login auth message). The derived Psy
   * identity is bound to this signature, so we verify it recovers to `address`
   * before returning — rejecting a malformed/mismatched signature BEFORE any
   * identity derivation or on-chain registration.
   */
  async personalSign(message: string, address: string): Promise<string> {
    let sig: string
    try {
      sig = await signMessage(this.wagmiConfig, { account: address as Hex, message })
    } catch (e) {
      if (isUserRejection(e)) throw new Error(REJECT_MESSAGE)
      throw e
    }
    let recovered: string
    try {
      recovered = verifyMessage(message, sig)
    } catch (e) {
      throw new Error(`Wallet returned a malformed signature: ${(e as Error)?.message ?? e}`)
    }
    if (recovered.toLowerCase() !== address.toLowerCase()) {
      throw new Error(
        `Signature verification failed: recovered ${recovered.toLowerCase()} != ${address.toLowerCase()}.`,
      )
    }
    return sig
  }

  /**
   * Mode-A per-tx auth: `personal_sign` the raw 32-byte Psy session sighash and
   * return the 65-byte signature (r‖s‖v). Byte-identical to the legacy
   * `personal_sign('0x' + sigHash)` — `signMessage({ message: { raw } })`
   * applies the same EIP-191 envelope over the same 32 bytes, which is what the
   * `EthPersonalSecp256K1` verifier circuit checks.
   *
   * WHY personal_sign and NOT raw eth_sign: raw `eth_sign` (blind-sign arbitrary
   * 32 bytes) is disabled/gated in many wallets (WalletConnect, Coinbase,
   * hardened MetaMask), which would break "any EVM wallet". personal_sign is
   * universally supported; the circuit was moved to the EIP-191 digest to match.
   */
  async ethPersonalSign(sigHash32Hex: string, address: string): Promise<string> {
    const raw = (sigHash32Hex.startsWith('0x') ? sigHash32Hex : '0x' + sigHash32Hex) as Hex
    try {
      return await signMessage(this.wagmiConfig, { account: address as Hex, message: { raw } })
    } catch (e) {
      if (isUserRejection(e)) throw new Error(REJECT_MESSAGE)
      throw e
    }
  }

  /**
   * `eth_signTypedData_v4` the domain-bound Auth struct (the v2 auth path).
   * wagmi/viem derive the `EIP712Domain` type from `domain`, so we pass only the
   * `Auth` type. Same recover-to-address integrity gate as `personalSign`.
   */
  async signTypedDataV4(
    domain: { name: string; version: string; chainId: number },
    types: { Auth: readonly Eip712Field[] },
    value: Record<string, unknown>,
    address: string,
  ): Promise<string> {
    let sig: string
    try {
      sig = await signTypedData(this.wagmiConfig, {
        account: address as Hex,
        domain,
        types: { Auth: types.Auth as Eip712Field[] },
        primaryType: 'Auth',
        message: value,
      })
    } catch (e) {
      if (isUserRejection(e)) throw new Error(REJECT_MESSAGE)
      throw e
    }
    let recovered: string
    try {
      // ethers' verifyTypedData wants the types object WITHOUT EIP712Domain.
      recovered = verifyTypedData(
        domain as TypedDataDomain,
        { Auth: types.Auth as Eip712Field[] },
        value,
        sig,
      )
    } catch (e) {
      throw new Error(
        `Wallet returned a malformed typed-data signature: ${(e as Error)?.message ?? e}`,
      )
    }
    if (recovered.toLowerCase() !== address.toLowerCase()) {
      throw new Error(
        `Signature verification failed: recovered ${recovered.toLowerCase()} != ${address.toLowerCase()}.`,
      )
    }
    return sig
  }

  /**
   * Subscribe to connected-account changes (connect / disconnect / switch).
   * Mirrors the old `accountsChanged` event via wagmi's `watchAccount`. Returns
   * an unsubscribe function.
   */
  onAccountsChanged(handler: (accounts: string[]) => void): () => void {
    return watchAccount(this.wagmiConfig, {
      onChange(account) {
        handler(account.address ? [account.address] : [])
      },
    })
  }
}
