import { hexToU8Array, u8ArrayToHex, hexToU8ArrayReversed } from "doge-sdk";
import {verify} from "@noble/secp256k1";

function normalizeSignatureFromDer(signatureHexDer: string) {
  const bytes = hexToU8Array(signatureHexDer);
  if(bytes[0]!==0x30){
    throw new Error("Invalid Signature");
  }
  const rLen = bytes[3];
  let r = u8ArrayToHex(bytes.slice(4, 4+rLen));
  const sLen = bytes[4+rLen+1];
  let s = u8ArrayToHex(bytes.slice(4+rLen+2));
  if(r.length>64){
    r = r.substring(2);
  }
  if(s.length>64){
    s = s.substring(2);
  }
  return r+s;
}
function verifySignature(signatureHex: string, messageHashHex: string, publicKeyHex: string): boolean {
  const r= BigInt("0x"+signatureHex.substring(0,64));
  const s= BigInt("0x"+signatureHex.substring(64));
  return verify({
    r,s
  }, messageHashHex, publicKeyHex);
}
function verifyNormalizeSecp256K1Signature(signatureHex: string, messageHashHex: string, publicKeyHex: string): string {
  if(signatureHex.length===64){
    if(verifySignature(signatureHex, messageHashHex, publicKeyHex)){
      return signatureHex;
    }else{
      throw new Error("Invalid Signature");
    }
  }else if(signatureHex.length>64 && signatureHex.substring(0,2)==="30"){
    const normalized = normalizeSignatureFromDer(signatureHex);
    if(verifySignature(normalized, messageHashHex, publicKeyHex)){
      return normalized;
    }else{
      throw new Error("Invalid Signature");
    }
  }else{
    throw new Error("Invalid Signature");
  }
}
export {
  verifyNormalizeSecp256K1Signature,
}