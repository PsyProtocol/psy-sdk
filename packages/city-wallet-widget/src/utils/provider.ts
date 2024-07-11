import { DogeMemoryWalletProvider, DogeNetworkId, FullDogeWalletProvider, IDogeTransactionSigner, IDogeWalletProvider, IFullDogeWalletProvider, TWalletAbility } from "doge-sdk";
import { IWidgetDogeWalletAdapter } from "../types";

class WidgetDogeWalletProvider<T extends IDogeWalletProvider> implements IDogeWalletProvider {
  innerProvider: T;
  fullProvider: IFullDogeWalletProvider<T>;
  constructor(provider: T) {
    this.innerProvider = provider;
    this.fullProvider = new FullDogeWalletProvider(provider);
  }
  addWalletRandom(networkId: DogeNetworkId): Promise<IDogeTransactionSigner> {
    if (this.innerProvider.addWalletRandom) {
      return this.innerProvider.addWalletRandom(networkId);
    } else {
      throw new Error("addWalletRandom not supported for this provider.");
    }
  }
  addWalletBIP39(networkId: DogeNetworkId, seedPhrase: string, password?: string | undefined): Promise<IDogeTransactionSigner> {
    if (this.innerProvider.addWalletBIP39) {
      return this.innerProvider.addWalletBIP39(networkId, seedPhrase, password);
    } else {
      throw new Error("addWalletRandom not supported for this provider.");
    }
  }
  addWalletBIP44(networkId: DogeNetworkId, fullDerivationPath: string): Promise<IDogeTransactionSigner> {
    if (this.innerProvider.addWalletBIP44) {
      return this.innerProvider.addWalletBIP44(networkId, fullDerivationPath);
    } else {
      throw new Error("addWalletBIP44 not supported for this provider.");
    }
  }
  addWalletBIP178(networkId: DogeNetworkId, wif: string): Promise<IDogeTransactionSigner> {
    if (this.innerProvider.addWalletBIP178) {
      return this.innerProvider.addWalletBIP178(networkId, wif);
    } else {
      throw new Error("addWalletBIP178 not supported for this provider.");
    }
  }
  getAbilities(): TWalletAbility[] {
    return this.innerProvider.getAbilities();
  }
  async getWalletPrivateKeyWIF(address: string): Promise<string> {
    if(this.innerProvider.getAbilities().includes("export-private-key-wif")){
      const signer = await this.fullProvider.getSignerForAddress(address);
      if(signer.getPrivateKeyWIF){
        const wif = await signer.getPrivateKeyWIF();
      }
    }
    throw new Error("Method not implemented for this provider.");
  }
  getCompressedPublicKeys(useCache?: boolean | undefined): Promise<string[]> {
    return this.fullProvider.getCompressedPublicKeys(useCache);
  }
  getSignerForPublicKey(compressedPublicKeyHex: string, useCache?: boolean | undefined): Promise<IDogeTransactionSigner> {
    return this.fullProvider.getSignerForPublicKey(compressedPublicKeyHex, useCache);
  }
  getP2PKHAddresses(networkId: string, useCache?: boolean | undefined): Promise<{ address: string; publicKey: string; }[]> {
    return this.fullProvider.getP2PKHAddresses(networkId, useCache);
  }
  getSignerForAddress(address: string, useCache?: boolean | undefined): Promise<IDogeTransactionSigner> {
    return this.fullProvider.getSignerForAddress(address, useCache);
  }
  getBaseProvider(): T {
    return this.innerProvider;
  }
  getSigners(): Promise<IDogeTransactionSigner[]> {
    return this.innerProvider.getSigners();
  }
  static fromMemoryProvider(provider: DogeMemoryWalletProvider): WidgetDogeWalletProvider<DogeMemoryWalletProvider> {
    return new WidgetDogeWalletProvider(provider);
  }
}



export {
  WidgetDogeWalletProvider
}