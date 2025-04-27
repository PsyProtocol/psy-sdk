use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::base_types::hash160::Hash160;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::qdata::checkpoint::QEDCheckpointLeaf;
use qed_data::qdata::checkpoint::QEDL2BlockState;
use qed_data::qdata::contract::ContractCodeDefinition;
use qed_data::qdata::contract::QEDContractLeaf;
use qed_data::qdata::user::QEDUserLeaf;
use qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData;
use qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition;
use qed_store::store::imm::cmd::QSRCmdGetContractLeafData;
use qed_store::store::imm::cmd::QSRCmdGetL2BlockState;
use qed_store::store::imm::cmd::QSRCmdGetUserLeafData;
use qed_store::store::imm::cmd::QSRHashCmd;
use qed_store::store::imm::cmd::QSRMerkleCmd;
use qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput;
use qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_with::serde_as;
use std::borrow::Cow;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Represents the version of the RPC protocol
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Version {
    #[serde(rename = "2.0")]
    V2,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(i64),
    Null,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(bound = "")]
#[serde(tag = "method", content = "params")]
pub enum RequestParams<F: RichField> {
    /// for coordinator edge
    #[serde(rename = "qed_deploy_contract")]
    DeployContract(QDeployContractRPCRequest<F>),
    #[serde(rename = "qed_register_user")]
    RegisterUser(QRegisterUserRPCRequest<F>),
    #[serde(rename = "qed_build_block")]
    ProduceBlock,
    #[serde(rename = "qed_get_user_id")]
    GetUserId(ZKPublicKeyInfo<F>),

    /// for realm edge
    TokenTransfer(QTokenTransferRPCRequest),
    #[serde(rename = "qed_claim_deposit")]
    ClaimDeposit(QClaimDepositRPCRequest),
    #[serde(rename = "qed_add_withdrawal")]
    AddWithdrawal(QAddWithdrawalRPCRequest),
    #[serde(rename = "qed_submit_user_end_cap")]
    SubmitEndCap(QSubmitEndCapRPCRequest<F>),

    // QTreeDataStoreReaderSync
    #[serde(rename = "qed_get_user_contract_state_tree_root")]
    GetUserContractStateTreeRoot(QUserContractStateTreeRootRPCRequest),
    #[serde(rename = "qed_get_user_contract_state_tree_root_f")]
    GetUserContractStateTreeRootF(QUserContractStateTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_user_contract_state_tree_leaf_hash")]
    GetUserContractStateTreeLeafHash(QUserContractStateTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_user_contract_state_tree_leaf_hash_f")]
    GetUserContractStateTreeLeafHashF(QUserContractStateTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_user_contract_state_tree_merkle_proof")]
    GetUserContractStateTreeMerkleProof(QUserContractStateTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_user_contract_state_tree_merkle_proof_f")]
    GetUserContractStateTreeMerkleProofF(QUserContractStateTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_user_contract_tree_root")]
    GetUserContractTreeRoot(QUserContractTreeRootRPCRequest),
    #[serde(rename = "qed_get_user_contract_tree_root_f")]
    GetUserContractTreeRootF(QUserContractTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_user_contract_tree_leaf_hash")]
    GetUserContractTreeLeafHash(QUserContractTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_user_contract_tree_leaf_hash_f")]
    GetUserContractTreeLeafHashF(QUserContractTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_user_contract_tree_merkle_proof")]
    GetUserContractTreeMerkleProof(QUserContractTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_user_contract_tree_merkle_proof_f")]
    GetUserContractTreeMerkleProofF(QUserContractTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_user_registration_tree_root")]
    GetUserRegistrationTreeRoot(QUserRegistrationTreeRootRPCRequest),
    #[serde(rename = "qed_get_user_registration_tree_root_f")]
    GetUserRegistrationTreeRootF(QUserRegistrationTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_user_registration_tree_leaf_hash")]
    GetUserRegistrationTreeLeafHash(QUserRegistrationTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_user_registration_tree_leaf_hash_f")]
    GetUserRegistrationTreeLeafHashF(QUserRegistrationTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_user_registration_tree_merkle_proof")]
    GetUserRegistrationTreeMerkleProof(QUserRegistrationTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_user_registration_tree_merkle_proof_f")]
    GetUserRegistrationTreeMerkleProofF(QUserRegistrationTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_user_tree_root")]
    GetUserTreeRoot(QUserTreeRootRPCRequest),
    #[serde(rename = "qed_get_user_tree_root_f")]
    GetUserTreeRootF(QUserTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_user_tree_leaf_hash")]
    GetUserTreeLeafHash(QUserTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_user_tree_leaf_hash_f")]
    GetUserTreeLeafHashF(QUserTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_user_tree_merkle_proof")]
    GetUserTreeMerkleProof(QUserTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_user_tree_merkle_proof_f")]
    GetUserTreeMerkleProofF(QUserTreeMerkleProofFRPCRequest<F>),
    #[serde(rename = "qed_get_user_sub_tree_merkle_proof")]
    GetUserSubTreeMerkleProof(QUserSubTreeMerkleProofRPCRequest),

    #[serde(rename = "qed_get_contract_function_tree_root")]
    GetContractFunctionTreeRoot(QContractFunctionTreeRootRPCRequest),
    #[serde(rename = "qed_get_contract_function_tree_root_f")]
    GetContractFunctionTreeRootF(QContractFunctionTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_function_tree_leaf_hash")]
    GetContractFunctionTreeLeafHash(QContractFunctionTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_contract_function_tree_leaf_hash_f")]
    GetContractFunctionTreeLeafHashF(QContractFunctionTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_function_tree_merkle_proof")]
    GetContractFunctionTreeMerkleProof(QContractFunctionTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_contract_function_tree_merkle_proof_f")]
    GetContractFunctionTreeMerkleProofF(QContractFunctionTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_contract_tree_root")]
    GetContractTreeRoot(QContractTreeRootRPCRequest),
    #[serde(rename = "qed_get_contract_tree_root_f")]
    GetContractTreeRootF(QContractTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_tree_leaf_hash")]
    GetContractTreeLeafHash(QContractTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_contract_tree_leaf_hash_f")]
    GetContractTreeLeafHashF(QContractTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_tree_merkle_proof")]
    GetContractTreeMerkleProof(QContractTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_contract_tree_merkle_proof_f")]
    GetContractTreeMerkleProofF(QContractTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_deposit_tree_root")]
    GetDepositTreeRoot(QDepositTreeRootRPCRequest),
    #[serde(rename = "qed_get_deposit_tree_root_f")]
    GetDepositTreeRootF(QDepositTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_deposit_tree_leaf_hash")]
    GetDepositTreeLeafHash(QDepositTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_deposit_tree_leaf_hash_f")]
    GetDepositTreeLeafHashF(QDepositTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_deposit_tree_merkle_proof")]
    GetDepositTreeMerkleProof(QDepositTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_deposit_tree_merkle_proof_f")]
    GetDepositTreeMerkleProofF(QDepositTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_withdrawal_tree_root")]
    GetWithdrawalTreeRoot(QWithdrawalTreeRootRPCRequest),
    #[serde(rename = "qed_get_withdrawal_tree_root_f")]
    GetWithdrawalTreeRootF(QWithdrawalTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_withdrawal_tree_leaf_hash")]
    GetWithdrawalTreeLeafHash(QWithdrawalTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_withdrawal_tree_leaf_hash_f")]
    GetWithdrawalTreeLeafHashF(QWithdrawalTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_withdrawal_tree_merkle_proof")]
    GetWithdrawalTreeMerkleProof(QWithdrawalTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_withdrawal_tree_merkle_proof_f")]
    GetWithdrawalTreeMerkleProofF(QWithdrawalTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_latest_checkpoint_tree_root")]
    GetLatestCheckpointTreeRoot(QLatestCheckpointTreeRootRPCRequest),
    #[serde(rename = "qed_get_checkpoint_tree_root")]
    GetCheckpointTreeRoot(QCheckpointTreeRootRPCRequest),
    #[serde(rename = "qed_get_checkpoint_tree_root_f")]
    GetCheckpointTreeRootF(QCheckpointTreeRootFRPCRequest<F>),
    #[serde(rename = "qed_get_checkpoint_tree_leaf_hash")]
    GetCheckpointTreeLeafHash(QCheckpointTreeLeafHashRPCRequest),
    #[serde(rename = "qed_get_checkpoint_tree_leaf_hash_f")]
    GetCheckpointTreeLeafHashF(QCheckpointTreeLeafHashFRPCRequest<F>),
    #[serde(rename = "qed_get_checkpoint_tree_merkle_proof")]
    GetCheckpointTreeMerkleProof(QCheckpointTreeMerkleProofRPCRequest),
    #[serde(rename = "qed_get_checkpoint_tree_merkle_proof_f")]
    GetCheckpointTreeMerkleProofF(QCheckpointTreeMerkleProofFRPCRequest<F>),

    #[serde(rename = "qed_get_checkpoint_global_state_roots")]
    GetCheckpointGlobalStateRoots(QCheckpointGlobalStateRootsRPCRequest),

    // QMetaDataStoreReaderSync
    #[serde(rename = "qed_get_user_leaf_data")]
    GetUserLeafData(QUserLeafDataRPCRequest),
    #[serde(rename = "get_user_leaf_data_f")]
    GetUserLeafFData(QUserLeafDataFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_leaf_data")]
    GetContractLeafData(QContractLeafDataRPCRequest),
    #[serde(rename = "get_contract_leaf_data_f")]
    GetContractLeafDataF(QContractLeafDataFRPCRequest<F>),
    #[serde(rename = "qed_get_checkpoint_leaf_data")]
    GetCheckpointLeafData(QCheckpointLeafDataRPCRequest),
    #[serde(rename = "get_checkpoint_leaf_data_f")]
    GetCheckpointLeafDataF(QCheckpointLeafDataFRPCRequest<F>),
    #[serde(rename = "qed_get_contract_code_definition")]
    GetContractCodeDefinition(QContractCodeDefinitionRPCRequest),
    #[serde(rename = "qed_get_contract_code_definition_f")]
    GetContractCodeDefinitionF(QContractCodeDefinitionFRPCRequest<F>),
    #[serde(rename = "qed_get_latest_l2_block_state")]
    GetLatestL2BlockState(QLatestL2BlockStateRPCRequest),
    #[serde(rename = "qed_get_l2_block_state")]
    GetL2BlockState(QL2BlockStateRPCRequest),
    #[serde(rename = "qed_get_l2_block_state_f")]
    GetL2BlockStateF(QL2BlockStateFRPCRequest<F>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(bound = "T: Serialize, for<'de2> T: Deserialize<'de2>")]
#[serde(deny_unknown_fields)]
pub struct RpcRequest<T> {
    /// The version of the protocol
    pub jsonrpc: Version,
    #[serde(flatten)]
    pub request: T,
    /// The name of the method to execute
    /// The identifier for this request issued by the client,
    /// An [Id] must be a String, null or a number.
    /// If missing it's considered a notification in [Version::V2]
    pub id: Id,
}

/// Response of a _single_ rpc call
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpcResponse<T> {
    // JSON RPC version
    pub jsonrpc: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    #[serde(flatten)]
    pub result: ResponseResult<T>,
}

/// Represents the result of a call either success or error
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ResponseResult<T> {
    #[serde(rename = "result")]
    Success(T),
    #[serde(rename = "error")]
    Error(RpcError),
}

type F = GoldilocksField;
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum LPSResponse {
    // Batch(QEDReadCommandBatchOutput<F>),
    // QTreeDataStoreReaderSync
    GetHash(QHashOut<F>),
    GetMerkleProof(MerkleProofCore<QHashOut<F>>),

    // QMetaDataStoreReaderSync
    GetUserLeaf(QEDUserLeaf<F>),
    GetContractLeaf(QEDContractLeaf<F>),
    GetContractCode(ContractCodeDefinition),
    GetCheckpointLeaf(QEDCheckpointLeaf<F>),
    GetL2BlockState(QEDL2BlockState),
    // GetLatestL2BlockState(QEDL2BlockState),
    GetUserId(u64),
}

/// Represents a JSON-RPC error
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    pub code: ErrorCode,
    /// error message
    pub message: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// List of JSON-RPC error codes
#[derive(Debug, Copy, PartialEq, Eq, Clone)]
pub enum ErrorCode {
    /// Server received Invalid JSON.
    /// server side error while parsing JSON
    ParseError,
    /// send invalid request object.
    InvalidRequest,
    /// method does not exist or valid
    MethodNotFound,
    /// invalid method parameter.
    InvalidParams,
    /// internal call error
    InternalError,
    /// Used for server specific errors.
    ServerError(i64),
}

impl ErrorCode {
    /// Returns the error code as `i64`
    pub fn code(&self) -> i64 {
        match *self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ServerError(c) => c,
        }
    }

    /// Returns the message associated with the error
    pub const fn message(&self) -> &'static str {
        match *self {
            ErrorCode::ParseError => "Parse error",
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::InvalidParams => "Invalid params",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::ServerError(_) => "Server error",
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}

impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<ErrorCode, D::Error>
    where
        D: Deserializer<'a>,
    {
        i64::deserialize(deserializer).map(Into::into)
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            _ => ErrorCode::ServerError(code),
        }
    }
}

impl From<ErrorCode> for RpcError {
    fn from(value: ErrorCode) -> Self {
        Self {
            code: value,
            message: Cow::Borrowed(value.message()),
            data: None,
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QTokenTransferRPCRequest {
    pub user_id: u64,
    pub to: u64,
    pub value: u64,
    pub nonce: u64,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QClaimDepositRPCRequest {
    pub user_id: u64,
    pub deposit_id: u64,
    pub value: u64,

    pub txid: Hash256,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub public_key: [u8; 33],

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAddWithdrawalRPCRequest {
    pub user_id: u64,
    pub value: u64,
    pub nonce: u64,

    pub destination_type: u8,
    pub destination: Hash160,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature_proof: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(transparent)]
pub struct QRegisterUserRPCRequest<F: RichField> {
    pub public_key: ZKPublicKeyInfo<F>,
}

impl<F: RichField> QRegisterUserRPCRequest<F> {
    pub fn new_batch(public_keys: &[ZKPublicKeyInfo<F>]) -> Vec<Self> {
        public_keys
            .iter()
            .map(|pk| QRegisterUserRPCRequest { public_key: *pk })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(transparent)]
pub struct QDeployContractRPCRequest<F: RichField> {
    pub deploy_contract: QBCDeployContract<F>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QSubmitEndCapRPCRequest<F: RichField> {
    pub user_ec_input: SubmitUserEndCapNonProofInput<F>,
    pub proof: ProofWithPublicInputs<GoldilocksField, C, D>,
}

// lps
// QTreeDataStoreReaderSync
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeRootRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
    pub contract_id: u32,
    pub height: u8,
    pub leaf_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub height: u8,
    pub leaf_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
    pub contract_id: u32,
    pub height: u8,
    pub leaf_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractStateTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
    pub height: u8,
    pub leaf_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeRootRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserContractTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub leaf_index: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub leaf_index: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub leaf_index: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserRegistrationTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub leaf_index: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserSubTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub root_level: u8,
    pub leaf_level: u8,
    pub leaf_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeRootRPCRequest {
    pub checkpoint_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub contract_id: u32,
    pub function_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub contract_id: F,
    pub function_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub contract_id: u32,
    pub function_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractFunctionTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub contract_id: F,
    pub function_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub contract_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub deposit_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub deposit_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub deposit_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QDepositTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub deposit_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub withdrawal_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub withdrawal_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub withdrawal_id: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QWithdrawalTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub withdrawal_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QLatestCheckpointTreeRootRPCRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeRootRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeRootFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeLeafHashRPCRequest {
    pub checkpoint_id: u64,
    pub leaf_checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeLeafHashFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub leaf_checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeMerkleProofRPCRequest {
    pub checkpoint_id: u64,
    pub leaf_checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointTreeMerkleProofFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub leaf_checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointGlobalStateRootsRPCRequest {
    pub checkpoint_id: u64,
}

// QMetaDataStoreReaderSync
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserLeafDataRPCRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QUserLeafDataFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
    pub user_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractLeafDataRPCRequest {
    pub contract_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractLeafDataFRPCRequest<F: RichField> {
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointLeafDataRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QCheckpointLeafDataFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractCodeDefinitionRPCRequest {
    pub contract_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QContractCodeDefinitionFRPCRequest<F: RichField> {
    pub contract_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QLatestL2BlockStateRPCRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QL2BlockStateRPCRequest {
    pub checkpoint_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QL2BlockStateFRPCRequest<F: RichField> {
    pub checkpoint_id: F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")]
#[serde(untagged)]
pub enum QRPCRequest<F: RichField> {
    QTokenTransferRPCRequest((u32, QTokenTransferRPCRequest)),
    QClaimDepositRPCRequest((u32, QClaimDepositRPCRequest)),
    QAddWithdrawalRPCRequest((u32, QAddWithdrawalRPCRequest)),
    QRegisterUserRPCRequest((u32, QRegisterUserRPCRequest<F>)),
    QDeployContractRPCRequest((u32, QDeployContractRPCRequest<F>)),
    QProduceBlockRPCRequest((u32, ())),
    QSubmitEndCapRPCRequest((u32, QSubmitEndCapRPCRequest<F>)),

    // QTreeDataStoreReaderSync
    QUserContractStateTreeRootRPCRequest((u32, QUserContractStateTreeRootRPCRequest)),
    QUserContractStateTreeRootFRPCRequest((u32, QUserContractStateTreeRootFRPCRequest<F>)),
    QUserContractStateTreeLeafHashRPCRequest((u32, QUserContractStateTreeLeafHashRPCRequest)),
    QUserContractStateTreeLeafHashFRPCRequest((u32, QUserContractStateTreeLeafHashFRPCRequest<F>)),
    QUserContractStateTreeMerkleProofRPCRequest((u32, QUserContractStateTreeMerkleProofRPCRequest)),
    QUserContractStateTreeMerkleProofFRPCRequest(
        (u32, QUserContractStateTreeMerkleProofFRPCRequest<F>),
    ),

    QUserContractTreeRootRPCRequest((u32, QUserContractTreeRootRPCRequest)),
    QUserContractTreeRootFRPCRequest((u32, QUserContractTreeRootFRPCRequest<F>)),
    QUserContractTreeLeafHashRPCRequest((u32, QUserContractTreeLeafHashRPCRequest)),
    QUserContractTreeLeafHashFRPCRequest((u32, QUserContractTreeLeafHashFRPCRequest<F>)),
    QUserContractTreeMerkleProofRPCRequest((u32, QUserContractTreeMerkleProofRPCRequest)),
    QUserContractTreeMerkleProofFRPCRequest((u32, QUserContractTreeMerkleProofFRPCRequest<F>)),
    QUserRegistrationTreeRootRPCRequest((u32, QUserRegistrationTreeRootRPCRequest)),
    QUserRegistrationTreeRootFRPCRequest((u32, QUserRegistrationTreeRootFRPCRequest<F>)),
    QUserRegistrationTreeLeafHashRPCRequest((u32, QUserRegistrationTreeLeafHashRPCRequest)),
    QUserRegistrationTreeLeafHashFRPCRequest((u32, QUserRegistrationTreeLeafHashFRPCRequest<F>)),
    QUserRegistrationTreeMerkleProofRPCRequest((u32, QUserRegistrationTreeMerkleProofRPCRequest)),
    QUserRegistrationTreeMerkleProofFRPCRequest(
        (u32, QUserRegistrationTreeMerkleProofFRPCRequest<F>),
    ),
    QUserTreeRootRPCRequest((u32, QUserTreeRootRPCRequest)),
    QUserTreeRootFRPCRequest((u32, QUserTreeRootFRPCRequest<F>)),
    QUserTreeLeafHashRPCRequest((u32, QUserTreeLeafHashRPCRequest)),
    QUserTreeLeafHashFRPCRequest((u32, QUserTreeLeafHashFRPCRequest<F>)),

    QUserSubTreeMerkleProofRPCRequest((u32, QUserSubTreeMerkleProofRPCRequest)),
    QContractFunctionTreeRootRPCRequest((u32, QContractFunctionTreeRootRPCRequest)),
    QContractFunctionTreeRootFRPCRequest((u32, QContractFunctionTreeRootFRPCRequest<F>)),
    QContractFunctionTreeLeafHashRPCRequest((u32, QContractFunctionTreeLeafHashRPCRequest)),
    QContractFunctionTreeLeafHashFRPCRequest((u32, QContractFunctionTreeLeafHashFRPCRequest<F>)),
    QContractFunctionTreeMerkleProofRPCRequest((u32, QContractFunctionTreeMerkleProofRPCRequest)),
    QContractFunctionTreeMerkleProofFRPCRequest(
        (u32, QContractFunctionTreeMerkleProofFRPCRequest<F>),
    ),
    QContractTreeRootRPCRequest((u32, QContractTreeRootRPCRequest)),
    QContractTreeRootFRPCRequest((u32, QContractTreeRootFRPCRequest<F>)),
    QContractTreeLeafHashRPCRequest((u32, QContractTreeLeafHashRPCRequest)),
    QContractTreeLeafHashFRPCRequest((u32, QContractTreeLeafHashFRPCRequest<F>)),
    QContractTreeMerkleProofRPCRequest((u32, QContractTreeMerkleProofRPCRequest)),
    QContractTreeMerkleProofFRPCRequest((u32, QContractTreeMerkleProofFRPCRequest<F>)),
    QDepositTreeRootRPCRequest((u32, QDepositTreeRootRPCRequest)),
    QDepositTreeRootFRPCRequest((u32, QDepositTreeRootFRPCRequest<F>)),
    QDepositTreeLeafHashRPCRequest((u32, QDepositTreeLeafHashRPCRequest)),
    QDepositTreeLeafHashFRPCRequest((u32, QDepositTreeLeafHashFRPCRequest<F>)),
    QDepositTreeMerkleProofRPCRequest((u32, QDepositTreeMerkleProofRPCRequest)),
    QDepositTreeMerkleProofFRPCRequest((u32, QDepositTreeMerkleProofFRPCRequest<F>)),
    QWithdrawalTreeRootRPCRequest((u32, QWithdrawalTreeRootRPCRequest)),
    QWithdrawalTreeRootFRPCRequest((u32, QWithdrawalTreeRootFRPCRequest<F>)),
    QWithdrawalTreeLeafHashRPCRequest((u32, QWithdrawalTreeLeafHashRPCRequest)),
    QWithdrawalTreeLeafHashFRPCRequest((u32, QWithdrawalTreeLeafHashFRPCRequest<F>)),
    QWithdrawalTreeMerkleProofRPCRequest((u32, QWithdrawalTreeMerkleProofRPCRequest)),
    QWithdrawalTreeMerkleProofFRPCRequest((u32, QWithdrawalTreeMerkleProofFRPCRequest<F>)),
    QLatestCheckpointTreeRootRPCRequest((u32, QLatestCheckpointTreeRootRPCRequest)),
    QCheckpointTreeRootRPCRequest((u32, QCheckpointTreeRootRPCRequest)),
    QCheckpointTreeRootFRPCRequest((u32, QCheckpointTreeRootFRPCRequest<F>)),
    QCheckpointTreeLeafHashRPCRequest((u32, QCheckpointTreeLeafHashRPCRequest)),
    QCheckpointTreeLeafHashFRPCRequest((u32, QCheckpointTreeLeafHashFRPCRequest<F>)),
    QCheckpointTreeMerkleProofRPCRequest((u32, QCheckpointTreeMerkleProofRPCRequest)),
    QCheckpointTreeMerkleProofFRPCRequest((u32, QCheckpointTreeMerkleProofFRPCRequest<F>)),
    QCheckpointGlobalStateRootsRPCRequest((u32, QCheckpointGlobalStateRootsRPCRequest)),

    // QMetaDataStoreReaderSync
    QUserLeafDataRPCRequest((u32, QUserLeafDataRPCRequest)),
    QUserLeafDataFRPCRequest((u32, QUserLeafDataFRPCRequest<F>)),
    QContractLeafDataRPCRequest((u32, QContractLeafDataRPCRequest)),
    QContractLeafDataFRPCRequest((u32, QContractLeafDataFRPCRequest<F>)),
    QCheckpointLeafDataRPCRequest((u32, QCheckpointLeafDataRPCRequest)),
    QCheckpointLeafDataFRPCRequest((u32, QCheckpointLeafDataFRPCRequest<F>)),
    QContractCodeDefinitionRPCRequest((u32, QContractCodeDefinitionRPCRequest)),
    QContractCodeDefinitionFRPCRequest((u32, QContractCodeDefinitionFRPCRequest<F>)),
    QLatestL2BlockStateRPCRequest((u32, QLatestL2BlockStateRPCRequest)),
    QL2BlockStateRPCRequest((u32, QL2BlockStateRPCRequest)),
    QL2BlockStateFRPCRequest((u32, QL2BlockStateFRPCRequest<F>)),
}
