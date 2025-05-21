import { Hash256 } from "@qed/qed-ts-sdk"
import { SCFelt } from "packages/city-sdk/src/rpc/baseTypes";

interface ICitySerializedTx {
  version: number;
  inputs: ICitySerializedTxInput[];
  outputs: ICitySerializedTxOutput[];
  locktime: number;
}
interface ICitySerializedTxInput {
    hash: Hash256;
    index: number;
    script: string;
    sequence: number;
}
interface ICitySerializedTxOutput {
    value: SCFelt;
    script: string;
}

interface ISigHashPreimage {
  transaction: ICitySerializedTx;
  sighash_type: number;
}

/*

    pub sighash_preimage: SigHashPreimage,

    pub last_block_spend_index: i32,
    pub block_spend_index: usize,

    pub current_spend_index: usize,

    pub funding_transactions: Vec<BTCTransaction>,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub next_block_redeem_script: Vec<u8>,
    */
interface IBlockSpendIntrospectionHint {
  sighash_preimage: ISigHashPreimage;
  last_block_spend_index: number;
  block_spend_index: number;
  current_spend_index: number;
  funding_transactions: ICitySerializedTx[];
  next_block_redeem_script: string;
}



export type {
  ICitySerializedTx,
  ICitySerializedTxInput,
  ICitySerializedTxOutput,
  ISigHashPreimage,
  IBlockSpendIntrospectionHint,
}