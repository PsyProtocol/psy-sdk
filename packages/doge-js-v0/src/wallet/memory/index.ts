import { Psbt } from "bitcoinjs-lib";
import { IDogeWalletProvider, ISecp256K1Provider } from "../../types/wallet";
import { DogeMemoryWallet } from "./wallet";
import { getSigHashForInput } from "../../helpers/signer";
import { DogeNetwork, IDogeWalletSerialized } from "../../types/network";

class DogeMemoryWalletProvider implements IDogeWalletProvider, ISecp256K1Provider {
  wallets: DogeMemoryWallet[];
  constructor(wallets: DogeMemoryWallet[] = []) {
    this.wallets = wallets;
  }
  addWalletFromWIF(wif: string, networkId: DogeNetwork, name?: string) {
    const wallet = DogeMemoryWallet.fromWIF(wif, networkId, name);
    this.wallets.push(wallet);
    return wallet;
  }
  addRandomWallet(networkId: DogeNetwork, name?: string) {
    const wallet = DogeMemoryWallet.generateRandom(networkId, name);
    this.wallets.push(wallet);
    return wallet;
  }
  addWallet(wallet: DogeMemoryWallet){
    this.wallets.push(wallet);
  }
  removeWallet(addressOrPublicKeyHex: string){
    this.wallets = this.wallets.filter(x=>x.keyPair.publicKey.toString("hex") !== addressOrPublicKeyHex && x.address !== addressOrPublicKeyHex);
  }
  getWalletForPublicKey(publicKey: string): DogeMemoryWallet | null {
    return this.wallets.find(x=>x.keyPair.publicKey.toString("hex") === publicKey) ?? null;
  }
  getWalletForPublicKeyOrThrow(publicKey: string): DogeMemoryWallet {
    const result = this.getWalletForPublicKey(publicKey);
    if(result === null){
      throw new Error("Wallet not found for public key "+publicKey);
    }
    return result;
  }
  canSignHash(): boolean {
    return true;
  }
  async signTransactionHash(compressedPublicKeyHex: string, hash: string): Promise<string> {
    const wallet = this.getWalletForPublicKeyOrThrow(compressedPublicKeyHex);
    return wallet.keyPair.sign(Buffer.from(hash, "hex")).toString("hex");
  }
  async signTransactionInput(compressedPublicKeyHex: string, psbt: Psbt, inputIndex: number, sigHashTypes: number[]): Promise<string> {
    const sigHash = getSigHashForInput(psbt.toBuffer(), inputIndex, sigHashTypes);
    const wallet = this.getWalletForPublicKeyOrThrow(compressedPublicKeyHex);
    return wallet.keyPair.sign(sigHash).toString("hex");
  }
  async getSecp256K1PublicKeys(): Promise<string[]> {
    return this.wallets.map(x=>x.keyPair.publicKey.toString("hex"));
  }
  async signMessageSecp256K1(compressedPublicKeyHex: string, message: string): Promise<string> {
    const wallet = this.getWalletForPublicKey(compressedPublicKeyHex);
    if(wallet === null){
      throw new Error("Wallet not found for public key "+compressedPublicKeyHex);
    }
    return wallet.signMessage(message);
  }
  async getPublicKeys(): Promise<string[]> {
    return this.wallets.map(x=>x.keyPair.publicKey.toString("hex"));
  }
  serialize(): IDogeWalletSerialized[] {
    return this.wallets.map(x=>x.serialize());
  }
  static deserialize(serializedWallets: IDogeWalletSerialized[]){
    return new DogeMemoryWalletProvider(serializedWallets.map(x=>DogeMemoryWallet.deserialize(x)));
  }
  toJSON() {
    return this.serialize();
  }

}

export {
  DogeMemoryWalletProvider,
  DogeMemoryWallet,
}