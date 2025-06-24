import { IQedCompleteUserInfo, IQedUserWallet } from "@qed/qed-sdk";

interface IQedWidgetWallet extends IQedCompleteUserInfo {
  wallet: IQedUserWallet;
}


export type { IQedWidgetWallet };