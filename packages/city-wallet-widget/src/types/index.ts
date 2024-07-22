import { ICityCompleteUserInfo, ICityUserWallet } from "@qstudio/city-sdk";
import { DogeNetworkId, IAddressStatsResponse, IDogeTransactionSigner, IDogeWalletProvider, IFullDogeWalletProvider, IUTXO } from "doge-sdk";

interface IQCityWidgetWallet extends ICityCompleteUserInfo {
  wallet: ICityUserWallet;
}


export type { IQCityWidgetWallet };