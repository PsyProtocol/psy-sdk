import { IQedCompleteUserInfo, IQedUserWallet } from "@qed/qed-sdk";

interface IQedWidgetWallet extends IQedCompleteUserInfo {
  wallet: IQedUserWallet;
  name: string;
  address: string;
}


export type { IQedWidgetWallet };