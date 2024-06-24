import * as ecc from '@bitcoinerlab/secp256k1';

import BIP32Factory from "bip32";
import ECPairFactory, { ECPairAPI } from "ecpair";

const bip32 = BIP32Factory(ecc);
const ECPair: ECPairAPI = ECPairFactory(ecc);


export {
  bip32,
  ECPair,
  ecc,
}