import { Psbt } from "bitcoinjs-lib";

interface ISimpleDogeWalletProvider {
  canSignHash(): boolean;
  signTransactionHash(compressedPublicKeyHex: string, hash: string): Promise<string>;
  signTransactionInput(compressedPublicKeyHex: string, psbt: Psbt, inputIndex: number, sigHashTypes: number[]): Promise<string>;
}
interface ISimpleDogeWalletSigner {
  canSignHash(): boolean;
  signTransactionHash(hash: string): Promise<string>;
  signTransactionInput(psbt: Psbt, inputIndex: number, sigHashTypes: number[]): Promise<string>;
}
interface IDogeWalletProvider extends ISimpleDogeWalletProvider {
  getPublicKeys(): Promise<string[]>;
}

interface ISecp256K1Provider {
  getSecp256K1PublicKeys(): Promise<string[]>;
  signMessageSecp256K1(compressedPublicKeyHex: string, message: string): Promise<string>;
}


export type {
  IDogeWalletProvider,
  ISimpleDogeWalletProvider,
  ISecp256K1Provider,
  ISimpleDogeWalletSigner,
}