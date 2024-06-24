import { Psbt, Signer, SignerAsync, Transaction } from "bitcoinjs-lib";
import { IDogeWalletProvider, ISimpleDogeWalletProvider, ISimpleDogeWalletSigner } from "../types/wallet";
import {Buffer} from "./buffer";
import { randomBytesInsecure } from "../utils/random";
class FakeSigner implements Signer {
  publicKey: Buffer = Buffer.from(new Uint8Array(33));
  network?: any;
  hash: Buffer = Buffer.from(new Uint8Array(32));
  sign(hash: Buffer, lowR?: boolean | undefined): Buffer {
    this.hash = hash;
    throw new Error("Method not implemented.");
  }
  signSchnorr?(hash: Buffer): Buffer {
    throw new Error("Method not implemented.");
  }
  getPublicKey?(): Buffer {
    return this.publicKey;
  }
}
function randomPublicKey(){
  const key = randomBytesInsecure(33);
  key[0] = 0x02;
  return Buffer.from(key);
}
class KnownSignatureSigner implements Signer {
  publicKey: Buffer = randomPublicKey();
  network?: any;
  hash: Buffer = Buffer.from(new Uint8Array(0));
  signature: Buffer;
  constructor(signature: Buffer){
    this.signature = signature;
  }
  sign(hash: Buffer, lowR?: boolean | undefined): Buffer {
    this.hash = hash;
    return this.signature;
  }
}
function getSigHashForInput(psbtBuffer: Buffer, inputIndex: number, sighashTypes?: number[]){
  const tmp = Psbt.fromBuffer(psbtBuffer);
  const fakeSigner = new FakeSigner();
  try{
    tmp.signInput(inputIndex, fakeSigner, sighashTypes);
  }catch(e){
    if(fakeSigner.hash.length === 0){
      throw e;
    }
  }
  return fakeSigner.hash;
}
function getSigHashesForPsbt(psbt: Psbt, sighashTypes: number[]){
  const buf = psbt.toBuffer();
  const inputHashes: Buffer[] = [];
  for(let i=0;i<psbt.inputCount;i++){
    inputHashes.push(getSigHashForInput(buf, i, sighashTypes));
  }
  return inputHashes;
}
class SimpleSigner implements SignerAsync {
  provider: ISimpleDogeWalletProvider;
  compressedPublicKeyHex: string;
  publicKey: Buffer;
  sigHashes: string[];
  psbt: Psbt;
  sighashTypes: number[];
  constructor(compressedPublicKeyHex: string, provider: ISimpleDogeWalletProvider, psbt: Psbt, sighashTypes: number[] = [Transaction.SIGHASH_ALL]) {
    this.provider = provider;
    this.compressedPublicKeyHex = compressedPublicKeyHex;
    this.publicKey = Buffer.from(compressedPublicKeyHex, "hex");
    this.psbt = psbt;
    this.sighashTypes = sighashTypes;
    this.sigHashes = getSigHashesForPsbt(psbt, sighashTypes).map(x=>x.toString("hex"));
  }
  async sign(hash: Buffer, lowR?: boolean | undefined): Promise<Buffer> {
    const hashStr = hash.toString("hex");
    const inputIndex = this.sigHashes.indexOf(hashStr);
    if(inputIndex === -1){
      throw new Error("Cannot find input index for hash");
    
    }
    const result = await this.provider.signTransactionInput(this.compressedPublicKeyHex, this.psbt, inputIndex, this.sighashTypes);
    return Buffer.from(result, "hex");
  }
  getPublicKey(): Buffer {
    return this.publicKey;
  }
}

class SimpleSingleSigner implements SignerAsync {
  walletSigner: ISimpleDogeWalletSigner;
  publicKey: Buffer = randomPublicKey();
  psbt: Psbt;
  sighashTypes: number[];
  sigHashes: string[];
  constructor(walletSigner: ISimpleDogeWalletSigner, psbt: Psbt, sighashTypes: number[] = [Transaction.SIGHASH_ALL]) {
    this.walletSigner = walletSigner;
    this.psbt = psbt;
    this.sighashTypes = sighashTypes;
    this.sigHashes = walletSigner.canSignHash()?[]:getSigHashesForPsbt(psbt, sighashTypes).map(x=>x.toString("hex"));
  }
  async sign(hash: Buffer, lowR?: boolean | undefined): Promise<Buffer> {
    if(this.walletSigner.canSignHash()){
      return Buffer.from(await this.walletSigner.signTransactionHash(hash.toString("hex")), "hex");
    }
    const hashStr = hash.toString("hex");
    const inputIndex = this.sigHashes.indexOf(hashStr);
    if(inputIndex === -1){
      throw new Error("Cannot find input index for hash");
    
    }
    const result = await this.walletSigner.signTransactionInput(this.psbt, inputIndex, this.sighashTypes);
    return Buffer.from(result, "hex");
  }
  getPublicKey(): Buffer {
    return this.publicKey;
  }
}

class SimpleHashSigner implements SignerAsync {
  provider: ISimpleDogeWalletProvider;
  compressedPublicKeyHex: string;
  publicKey: Buffer;
  constructor(compressedPublicKeyHex: string, provider: ISimpleDogeWalletProvider) {
    this.provider = provider;
    this.compressedPublicKeyHex = compressedPublicKeyHex;
    this.publicKey = Buffer.from(compressedPublicKeyHex, "hex");
  }
  async sign(hash: Buffer, lowR?: boolean | undefined): Promise<Buffer> {
    const result = await this.provider.signTransactionHash(this.compressedPublicKeyHex, hash.toString("hex"));
    return Buffer.from(result, "hex");
  }
  getPublicKey(): Buffer {
    return this.publicKey;
  }

}
function getSignerFromWalletProvider(compressedPublicKeyHex: string, provider: ISimpleDogeWalletProvider, psbt: Psbt, sighashTypes: number[] = [Transaction.SIGHASH_ALL]): SignerAsync{
  if(provider.canSignHash()){
    return new SimpleHashSigner(compressedPublicKeyHex, provider);
  }else{
    return new SimpleSigner(compressedPublicKeyHex, provider, psbt, sighashTypes);
  }
}
function getLinkSignerFromWalletProvider(compressedPublicKeyHex: string, provider: ISimpleDogeWalletProvider){
  return new SimpleHashSigner(compressedPublicKeyHex, provider);
}
function getLinkSignerFromKnownSignature(signature: string | Buffer){
  const realSignature = typeof signature === "string" ? Buffer.from(signature, "hex") : signature;
  return new KnownSignatureSigner(realSignature);
}

function getLinkSignerFromWalletSigner(walletSigner: ISimpleDogeWalletSigner, psbt: Psbt, sighashTypes: number[] = [Transaction.SIGHASH_ALL]){
  return new SimpleSingleSigner(walletSigner, psbt, sighashTypes);
}
export {
  getSignerFromWalletProvider,
  getSigHashForInput,
  getLinkSignerFromWalletProvider,
  getLinkSignerFromKnownSignature,
  getLinkSignerFromWalletSigner,
}