import { IDogeLinkElectrsRPC } from "doge-sdk/dist/types";

interface IWalletWidgetRPC extends IDogeLinkElectrsRPC {
  sendFromWallet(
    address: string,
    amount: number | string,
    walletName?: string
  ): Promise<string>;
  canSendFromWallet(): boolean;
}

export type { IWalletWidgetRPC };