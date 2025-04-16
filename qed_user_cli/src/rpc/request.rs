use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::base_types::hash160::Hash160;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
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
    #[serde(rename = "cr_deploy_contract")]
    DeployContract(QDeployContractRPCRequest<F>),
    TokenTransfer(QTokenTransferRPCRequest),
    #[serde(rename = "cr_claim_deposit")]
    ClaimDeposit(QClaimDepositRPCRequest),
    #[serde(rename = "cr_add_withdrawal")]
    AddWithdrawal(QAddWithdrawalRPCRequest),
    #[serde(rename = "cr_register_user")]
    RegisterUser(QRegisterUserRPCRequest<F>),
    #[serde(rename = "cr_produce_block")]
    ProduceBlock,
    #[serde(rename = "cr_get_block")]
    SubmitEndCap(QSubmitEndCapRPCRequest<F>),
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
    pub public_key: QHashOut<F>,
}

impl<F: RichField> QRegisterUserRPCRequest<F> {
    pub fn new_batch(public_keys: &[QHashOut<F>]) -> Vec<Self> {
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
}
