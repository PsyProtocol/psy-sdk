import { IPsyCompleteUserInfo, IPsyUserWallet } from "@psy/psy-sdk";

interface IPsyWidgetWallet extends IPsyCompleteUserInfo {
  wallet: IPsyUserWallet;
  name: string;
  address: string;
  isActive: boolean;
}

export const DEFAULT_WALLET_NAME = "******";


export type { IPsyWidgetWallet };