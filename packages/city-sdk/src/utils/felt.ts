import { SCNumberLike } from "../rpc/baseTypes";

const GOLDILOCKS_FP = BigInt("18446744069414584321");
function cityFelt(x: SCNumberLike): bigint {
  return BigInt(x) % GOLDILOCKS_FP;
}

export {
  cityFelt,
}