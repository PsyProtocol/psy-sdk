import { DogeNetworkId, IAddressStatsResponse, IDogeTransactionSigner, IDogeWalletProvider, IFullDogeWalletProvider, IUTXO } from "doge-sdk";

interface IQWidgetWallet {
  networkId: DogeNetworkId;
  address: string;
  balance: number;
  confirmedBalance: number;
  stats:IAddressStatsResponse
  signer: IDogeTransactionSigner;
  utxos: IUTXO[];
}

interface IWidgetDogeWalletAdapter<T extends IDogeWalletProvider> {
  addWalletFromWIF: (provider: T, wif: string) => Promise<IDogeTransactionSigner>;
  addNewWallet: (provider: T, networkId: DogeNetworkId) => Promise<IDogeTransactionSigner>;

}

export type { IQWidgetWallet, IWidgetDogeWalletAdapter };