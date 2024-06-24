
import type { PsbtInput, PsbtOutput } from "bip174/src/lib/interfaces";
import type { TransactionInput } from "bitcoinjs-lib/src/psbt";
import type {Buffer} from '../helpers/buffer';
import { ISimpleDogeWalletSigner } from "./wallet";

type DogeNetwork = "doge" | "dogeTestnet" | "dogeRegtest";

interface IDogeLinkRPCInfo {
  network: DogeNetwork;
  url: string;
  fullUrl: string;
  user?: string;
  password?: string;
}


interface IBaseUTXO {
  txid: string;
  vout: number;
  value: number;
}
interface IUTXO extends IBaseUTXO {
  status: {
      confirmed: boolean;
      block_height: number;
      block_hash: string;
      block_time: number;
  };
}

interface IUTXOWithRawTransaction extends IUTXO {
  rawTransaction: Buffer;
}

interface IFundingUTXO extends IBaseUTXO {
  rawTransaction: Buffer | string;
  signers?: ISimpleDogeWalletSigner[];
}
interface IFinalizedFundingUTXO extends IBaseUTXO {
  rawTransaction: Buffer;
  signers?: ISimpleDogeWalletSigner[];
  redeemScript?: Buffer;
}


interface IUTXOReference {
  txid: string;
  vouts?: number[];
}


interface IPayToScriptHashUTXO {
  type: "p2sh",
  address: string;
  publicKey: Buffer;
  redeemScript: Buffer;
}


interface IDogeWalletSerialized {
  wif: string;
  networkId: DogeNetwork;
  name: string;
}


interface IPsbtInputExtended extends PsbtInput, TransactionInput {
}
type ISpendOutput = PsbtOutputExtendedAddress | PsbtOutputExtendedScript | PsbtOutputExtendedScriptString;

type IPsbtOutputExtended = PsbtOutputExtendedAddress | PsbtOutputExtendedScript;
interface PsbtOutputExtendedAddress extends PsbtOutput {
    address: string;
    value: number;
}
interface PsbtOutputExtendedScript extends PsbtOutput {
    script: Buffer;
    value: number;
}
interface PsbtOutputExtendedScriptString extends PsbtOutput {
    script: string;
    value: number;
}

export type {
  DogeNetwork,
  IDogeLinkRPCInfo,
  IUTXO,
  IUTXOWithRawTransaction,
  IPayToScriptHashUTXO,
  IDogeWalletSerialized,
  IPsbtInputExtended,
  IPsbtOutputExtended,
  PsbtOutputExtendedAddress,
  PsbtOutputExtendedScript,
  IFundingUTXO,
  IBaseUTXO,
  ISpendOutput,
  IUTXOReference,
  IFinalizedFundingUTXO,
}