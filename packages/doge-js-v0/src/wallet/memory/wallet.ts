import { Network, payments } from "bitcoinjs-lib";
import { ECPairInterface } from "ecpair";
import { DogeNetwork, IDogeWalletSerialized } from "../../types/network";
import { getNetworkById } from "../../utils/networks";
import { ECPair } from "../../helpers/ecc";

class DogeMemoryWallet {
  keyPair: ECPairInterface;
  address: string;
  wif: string;
  networkId: DogeNetwork;
  name: string;
  constructor(keyPair: ECPairInterface, address: string, network: DogeNetwork, name?: string){
    this.keyPair = keyPair;
    this.address = address;
    this.networkId = network;
    this.wif = keyPair.toWIF();
    this.name = name ? name : "My Wallet";
  }

  getId(){
    return this.networkId+"`"+this.wif;
  }
  getCompressedPublicKeyHex() {
    return this.keyPair.publicKey.toString("hex");
  }

  getNetwork(): Network {
    return getNetworkById(this.networkId);
  }

  static fromWIF(wif: string, networkId: DogeNetwork, name?: string){
    const network = getNetworkById(networkId);
    const keyPair = ECPair.fromWIF(wif, network);
    const { address } = payments.p2pkh({
      pubkey: keyPair.publicKey,
      network,
    });
    if(!address){
      throw new Error("error generating wallet address from WIF");
    }
    return new DogeMemoryWallet(keyPair, address, networkId, name);
  }
  setNetworkId(networkId: DogeNetwork){
    this.networkId = networkId;
    if(this.keyPair.privateKey){
      this.keyPair = ECPair.fromPrivateKey(this.keyPair.privateKey, { network: getNetworkById(networkId) });
      this.wif = this.keyPair.toWIF();

      const { address } = payments.p2pkh({
        pubkey: this.keyPair.publicKey,
        network: getNetworkById(networkId),
      });
      if(!address){
        throw new Error("error generating wallet address");
      }
      this.address = address;
    }else{
      throw new Error("cannot set network id on key");
    }
  }

  static generateRandom(networkId: DogeNetwork, name?: string){
    const network = getNetworkById(networkId);
    const keyPair = ECPair.makeRandom({ network });
    const { address } = payments.p2pkh({
      pubkey: keyPair.publicKey,
      network,
    });
    if(!address){
      throw new Error("error generating random wallet address");
    }
    return new DogeMemoryWallet(keyPair, address, networkId, name);
  }

  serialize(): IDogeWalletSerialized {
    return {
      wif: this.wif,
      networkId: this.networkId,
      name: this.name,
    }
  }
  static deserialize(data: IDogeWalletSerialized){
    return DogeMemoryWallet.fromWIF(data.wif, data.networkId, data.name);
  }
  signMessage(message: string){
    return this.keyPair.sign(Buffer.from(message)).toString("hex");
  }
  toJSON(){
    return this.serialize();
  }
}

export {
  DogeMemoryWallet,
}