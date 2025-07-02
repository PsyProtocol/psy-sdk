import { IQedCompleteUserInfo, IQedUserWallet } from "@qed/qed-sdk";

interface IQedWidgetWallet extends IQedCompleteUserInfo {
  wallet: IQedUserWallet;
  name: string;
  address: string;
  isActive: boolean;
}

export const DEFAULT_WALLET_NAME = "******";


export type { IQedWidgetWallet };