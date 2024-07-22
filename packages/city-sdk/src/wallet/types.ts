import { DogeNetworkId } from "doge-sdk";
import { ICityTransactionSigner, ICityTransactionSignerProvider } from "../zksigner/types";
import { Hash256, HexString, ICityAddWithdrawalRPCRequest, ICityClaimDepositRPCRequest, ICityL1Deposit, ICityTokenTransferRPCRequest, SCNumberLike } from "../rpc/baseTypes";
import { ICitySecp256K1SignatureProver } from "../userProverRPC/types";
interface ICoreCityUserInfo {
  networkId: DogeNetworkId;
  l2NetworkMagic: string;
  userId: number;
  publicKeyHex: string;
}
interface ICityCompleteUserInfo extends ICoreCityUserInfo {
  nonce: string;
  balance: bigint;
}


interface ICityUserWallet extends ICoreCityUserInfo {
  signer: ICityTransactionSigner;
  getUserInfo(): Promise<ICityCompleteUserInfo>;
  getBalance(): Promise<bigint>;
  getBalanceString(): Promise<string>;
  getClaimDepositMessageHash(txidOrDepositId: string | number): Promise<{hash: string, deposit: ICityL1Deposit}>;
  prepareTransfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<ICityTokenTransferRPCRequest>;
  prepareWithdrawal(l1Address: string, amount: SCNumberLike, nonce?: SCNumberLike): Promise<ICityAddWithdrawalRPCRequest>;
  prepareClaimDeposit(txidOrDepositId: Hash256 | number, signature: HexString, prover: ICitySecp256K1SignatureProver): Promise<ICityClaimDepositRPCRequest>;
  transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
  withdraw(l1Address: string, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
  claimDeposit(txidOrDepositId: Hash256 | number, signature: HexString, prover: ICitySecp256K1SignatureProver): Promise<void>;
}
interface ICityUserWalletProvider {
  networkId: DogeNetworkId;
  l2NetworkMagic: string;
  signerProvider: ICityTransactionSignerProvider;
  getUserWallets(): Promise<ICityUserWallet[]>;
}

export type {
  ICoreCityUserInfo,
  ICityCompleteUserInfo,
  ICityUserWallet,
  ICityUserWalletProvider,
}