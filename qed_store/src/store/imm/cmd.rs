use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRCmdGetUserLeafData {
    pub checkpoint_id: u64,
    pub user_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRCmdGetContractLeafData {
    pub contract_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRCmdGetContractCodeDefinition {
    pub contract_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRCmdGetCheckpointLeafData {
    pub checkpoint_id: u64,
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRCmdGetL2BlockState {
    pub checkpoint_id: u64,
}


// start tree hash cmds

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserContractStateTreeRoot {
  pub checkpoint_id: u64,
  pub user_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserContractStateTreeLeafHash {
  pub checkpoint_id: u64,
  pub user_id: u64,
  pub contract_id: u32,
  pub height: u8,
  pub leaf_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserContractTreeRoot {
  pub checkpoint_id: u64,
  pub user_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserContractTreeLeafHash {
  pub checkpoint_id: u64,
  pub user_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserTreeRoot {
  pub checkpoint_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetUserTreeLeafHash {
  pub checkpoint_id: u64,
  pub user_id: u64,
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetContractFunctionTreeRoot {
  pub checkpoint_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetContractFunctionTreeLeafHash {
  pub checkpoint_id: u64,
  pub contract_id: u32,
  pub function_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetContractTreeRoot {
  pub checkpoint_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetContractTreeLeafHash {
  pub checkpoint_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetDepositTreeRoot {
  pub checkpoint_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetDepositTreeLeafHash {
  pub checkpoint_id: u64,
  pub deposit_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetWithdrawalTreeRoot {
  pub checkpoint_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetWithdrawalTreeLeafHash {
  pub checkpoint_id: u64,
  pub withdrawal_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetCheckpointTreeRoot {
  pub checkpoint_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRHashCmdGetCheckpointTreeLeafHash {
  pub checkpoint_id: u64,
  pub leaf_checkpoint_id: u32
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum QSRHashCmd {
    GetUserContractStateTreeRoot(QSRHashCmdGetUserContractStateTreeRoot),
    GetUserContractStateTreeLeafHash(QSRHashCmdGetUserContractStateTreeLeafHash),
    GetUserContractTreeRoot(QSRHashCmdGetUserContractTreeRoot),
    GetUserContractTreeLeafHash(QSRHashCmdGetUserContractTreeLeafHash),
    GetUserTreeRoot(QSRHashCmdGetUserTreeRoot),
    GetUserTreeLeafHash(QSRHashCmdGetUserTreeLeafHash),
    GetContractFunctionTreeRoot(QSRHashCmdGetContractFunctionTreeRoot),
    GetContractFunctionTreeLeafHash(QSRHashCmdGetContractFunctionTreeLeafHash),
    GetContractTreeRoot(QSRHashCmdGetContractTreeRoot),
    GetContractTreeLeafHash(QSRHashCmdGetContractTreeLeafHash),
    GetDepositTreeRoot(QSRHashCmdGetDepositTreeRoot),
    GetDepositTreeLeafHash(QSRHashCmdGetDepositTreeLeafHash),
    GetWithdrawalTreeRoot(QSRHashCmdGetWithdrawalTreeRoot),
    GetWithdrawalTreeLeafHash(QSRHashCmdGetWithdrawalTreeLeafHash),
    GetCheckpointTreeRoot(QSRHashCmdGetCheckpointTreeRoot),
    GetCheckpointTreeLeafHash(QSRHashCmdGetCheckpointTreeLeafHash),
}
// end tree hash cmds

// start tree merkle proof cmds

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetUserContractStateTreeMerkleProof {
  pub checkpoint_id: u64,
  pub user_id: u64,
  pub contract_id: u32,
  pub height: u8,
  pub leaf_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetUserContractTreeMerkleProof {
  pub checkpoint_id: u64,
  pub user_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetUserTreeMerkleProof {
  pub checkpoint_id: u64,
  pub user_id: u64
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetContractFunctionTreeMerkleProof {
  pub checkpoint_id: u64,
  pub contract_id: u32,
  pub function_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetContractTreeMerkleProof {
  pub checkpoint_id: u64,
  pub contract_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetDepositTreeMerkleProof {
  pub checkpoint_id: u64,
  pub deposit_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetWithdrawalTreeMerkleProof {
  pub checkpoint_id: u64,
  pub withdrawal_id: u32
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct QSRMerkleCmdGetCheckpointTreeMerkleProof {
  pub checkpoint_id: u64,
  pub leaf_checkpoint_id: u32
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum QSRMerkleCmd {
    GetUserContractStateTreeMerkleProof(QSRMerkleCmdGetUserContractStateTreeMerkleProof),
    GetUserContractTreeMerkleProof(QSRMerkleCmdGetUserContractTreeMerkleProof),
    GetUserTreeMerkleProof(QSRMerkleCmdGetUserTreeMerkleProof),
    GetContractFunctionTreeMerkleProof(QSRMerkleCmdGetContractFunctionTreeMerkleProof),
    GetContractTreeMerkleProof(QSRMerkleCmdGetContractTreeMerkleProof),
    GetDepositTreeMerkleProof(QSRMerkleCmdGetDepositTreeMerkleProof),
    GetWithdrawalTreeMerkleProof(QSRMerkleCmdGetWithdrawalTreeMerkleProof),
    GetCheckpointTreeMerkleProof(QSRMerkleCmdGetCheckpointTreeMerkleProof),
}
// end tree merkle proof cmds

