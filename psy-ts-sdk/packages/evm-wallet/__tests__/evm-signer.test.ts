import { EvmSigner } from '../src/evm/signer'
import { getAccount } from 'wagmi/actions'
import { Wallet } from 'ethers'
import type { Config } from 'wagmi'

jest.mock('wagmi/actions', () => ({ getAccount: jest.fn() }))

const domain = { name: 'Auth', version: '2', chainId: 1 }
const types = { Auth: [{ name: 'account', type: 'address' }] }
const wallet = new Wallet('0x' + '11'.repeat(32))
const value = { account: wallet.address }

function setup(sign: () => Promise<string>) {
  const request = jest.fn(async ({ method, params }: { method: string; params?: unknown[] }) => {
    if (method === 'eth_chainId') return '0xaa36a7'
    if (method === 'wallet_switchEthereumChain') {
      if ((params?.[0] as { chainId: string }).chainId === '0xaa36a7') {
        throw new Error('Network restoration failed')
      }
      return null
    }
    if (method === 'eth_signTypedData_v4') return sign()
    throw new Error(`Unexpected method: ${method}`)
  })
  jest.mocked(getAccount).mockReturnValue({
    address: wallet.address,
    connector: { getProvider: async () => ({ request }) },
  } as unknown as ReturnType<typeof getAccount>)
  return { signer: new EvmSigner({} as Config), request }
}

test('returns the verified signature when restoring the network fails', async () => {
  const signature = await wallet.signTypedData(domain, types, value)
  const { signer, request } = setup(async () => signature)
  await expect(signer.signTypedDataV4(domain, types, value, wallet.address)).resolves.toBe(signature)
  expect(request).toHaveBeenLastCalledWith({
    method: 'wallet_switchEthereumChain', params: [{ chainId: '0xaa36a7' }],
  })
})

test('preserves the signing error when restoring the network also fails', async () => {
  const error = new Error('Signing failed')
  const { signer } = setup(async () => { throw error })
  await expect(signer.signTypedDataV4(domain, types, value, wallet.address)).rejects.toBe(error)
})

test('still verifies the signature when restoring the network fails', async () => {
  const { signer } = setup(async () => '0x1234')
  await expect(signer.signTypedDataV4(domain, types, value, wallet.address))
    .rejects.toThrow('Wallet returned a malformed typed-data signature')
})
