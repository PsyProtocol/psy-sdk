
type SCNumberLike = bigint | number | string;
type SCFelt = bigint | number;
type HexString = string;

type CityHash = string;
type Hash256 = string;
type Hash160 = string;
type CompressedPublicKeyHex = string;
type QProvingJobDataIDSerializedWrapped = string;

interface IMerkleProofCore<T> {
  root: T;
  value: T;
  index: SCFelt;
  siblings: T[];
}

interface IDeltaMerkleProofCore<T> {
  old_root: T;
  old_value: T;
  new_root: T;
  new_value: T;
  index: SCFelt;
  siblings: T[];
}


type CityMerkleProof = IMerkleProofCore<CityHash>;
type CityDeltaMerkleProof = IDeltaMerkleProofCore<CityHash>;
interface ICityUserState {
  user_id: number;
  balance: SCFelt;
  nonce: SCFelt;
  alt_0: number;
  alt_1: number;
  public_key: CityHash;
}

interface ICityL1Deposit {
  deposit_id: number;
  checkpoint_id: number;
  value: SCFelt;
  txid: Hash256;
  public_key: CompressedPublicKeyHex;
}


interface ICityL2BlockState {
  checkpoint_id: number;
  next_add_withdrawal_id: number;
  next_process_withdrawal_id: number;
  next_deposit_id: number;
  total_deposits_claimed_epoch: number;
  next_user_id: number;
  end_balance: SCFelt;
}

interface ICityL1Withdrawal {
  withdrawal_id: number;
  address: Hash160;
  address_type: number;
  value: SCFelt;
}

interface ISimpleKVPair<K, V>{
  key: K;
  value: V;
}


interface ICityRegisterUserRPCRequest {
  public_key: CityHash;
}



interface ICityAddWithdrawalRPCRequest {
  user_id: number;
  value: SCFelt;
  nonce: SCFelt;
  destination_type: number;
  destination: Hash160;
  signature_proof: string;
}

interface ICityClaimDepositRPCRequest {
  user_id: number;
  deposit_id: number;
  value: SCFelt;
  txid: Hash256;
  public_key: CompressedPublicKeyHex;
  signature_proof: string;
}


interface ICityTokenTransferRPCRequest {
  user_id: number;
  to: number;
  value: SCFelt;
  nonce: SCFelt;
  signature_proof: string;
}

type TProofValueStoreKV = ISimpleKVPair<string, string>;


export type {
  CityHash,
  SCFelt,
  SCNumberLike,
  Hash256,
  Hash160,
  CompressedPublicKeyHex,
  QProvingJobDataIDSerializedWrapped,
  CityMerkleProof,
  CityDeltaMerkleProof,
  ICityUserState,
  ICityL1Deposit,
  ICityL2BlockState,
  ICityL1Withdrawal,
  ISimpleKVPair,
  ICityRegisterUserRPCRequest,
  ICityAddWithdrawalRPCRequest,
  ICityClaimDepositRPCRequest,
  ICityTokenTransferRPCRequest,
  TProofValueStoreKV,
  HexString,
}