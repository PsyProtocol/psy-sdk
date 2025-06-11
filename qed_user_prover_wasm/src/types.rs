//! Type definitions for QED User Prover WASM module

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Proof structure compatible with plonky2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct Proof {
    #[wasm_bindgen(getter_with_clone)]
    pub wires_cap: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub plonk_zs_partial_products_cap: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub quotient_polys_cap: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub openings: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub opening_proof: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct ProofOpenings {
    #[wasm_bindgen(getter_with_clone)]
    pub constants: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub plonk_sigmas: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub wires: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub plonk_zs: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub plonk_zs_next: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub partial_products: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub quotient_polys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct FriProof {
    #[wasm_bindgen(getter_with_clone)]
    pub commit_phase_merkle_caps: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub query_round_proofs: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub final_poly: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub pow_witness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct FriQueryRound {
    #[wasm_bindgen(getter_with_clone)]
    pub initial_trees_proof: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct FriQueryStep {
    #[wasm_bindgen(getter_with_clone)]
    pub evals: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub merkle_proof: Vec<String>,
}

/// Proof with public inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ProofWithPublicInputs {
    pub proof: Proof,
    pub public_inputs: Vec<String>,
}

/// DPN Assert Eq Info Indexed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct DPNAssertEqInfoIndexed {
    pub lhs: u32,
    pub rhs: u32,
}

/// DPN Indexed Var Def
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct DPNIndexedVarDef {
    pub var_name: String,
    pub var_index: u32,
}

/// DPN State Command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct DPNStateCmd {
    pub cmd_type: String,
    pub params: Vec<String>,
}

/// DPN Function Circuit Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct DPNFunctionCircuitDefinition {
    pub function_name: String,
    pub input_vars: Vec<DPNIndexedVarDef>,
    pub output_vars: Vec<DPNIndexedVarDef>,
    pub assert_eq_infos: Vec<DPNAssertEqInfoIndexed>,
    pub state_cmds: Vec<DPNStateCmd>,
}

/// Contract Code Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ContractCodeDefinition {
    pub contract_name: String,
    pub functions: Vec<ContractFunctionCodeDefinition>,
}

/// Contract Function Code Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ContractFunctionCodeDefinition {
    pub function_name: String,
    pub circuit_def: DPNFunctionCircuitDefinition,
}

/// Submit User End Cap Non Proof Core Input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct SubmitUserEndCapNonProofCoreInput {
    pub user_id: String,
    pub contract_address: String,
    pub function_name: String,
    pub inputs: Vec<String>,
    pub timestamp: u64,
}

/// QED Contract State Update History
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct QEDContractStateUpdateHistory {
    pub contract_address: String,
    pub updates: Vec<StateUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct StateUpdate {
    pub block_number: u64,
    pub transaction_hash: String,
    pub state_root: String,
}

/// Submit User End Cap Non Proof Input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct SubmitUserEndCapNonProofInput {
    pub core_input: SubmitUserEndCapNonProofCoreInput,
    pub state_history: QEDContractStateUpdateHistory,
    pub additional_data: Option<String>,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: Option<String>,
    pub created_at: u64,
    pub last_activity: u64,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct UserInfo {
    pub user_id: String,
    pub public_key: String,
    pub created_at: u64,
}

/// Keypair information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct KeypairInfo {
    pub public_key: String,
    pub private_key: String,
}

/// Contract deployment command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct DeployContractCmd {
    pub contract_code: String,
    pub constructor_args: Vec<String>,
    pub gas_limit: u64,
}

/// Signature hash information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct SigHashInfo {
    pub hash: String,
    pub message: String,
}

/// ZK Signature
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ZKSignature {
    pub signature: String,
    pub public_key: String,
    pub message_hash: String,
}

/// Result wrapper for async operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct AsyncResult {
    pub id: String,
    pub status: String, // "pending", "completed", "failed"
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Contract call arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct ContractCallArgs {
    pub contract_address: String,
    pub function_name: String,
    pub inputs: Vec<String>,
    pub gas_limit: Option<u64>,
}

/// RPC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct RpcConfig {
    pub rpc_url: String,
    pub api_key: Option<String>,
    pub encryption_key: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            api_key: None,
            encryption_key: None,
            timeout_ms: 30000,
            max_retries: 3,
        }
    }
}

/// Hash256 wrapper for WASM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Hash256 {
    pub bytes: [u8; 32],
}

impl Hash256 {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
    }
}

/// U8Bytes wrapper for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct U8Bytes(pub Vec<u8>);

/// QHashOut wrapper for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QHashOutWrapper {
    pub elements: [String; 4],
}

impl QHashOutWrapper {
    pub fn from_qhashout<F: plonky2::field::types::Field>(qhash: &qed_core::data::qhashout::QHashOut<F>) -> Self {
        Self {
            elements: [
                qhash.0.elements[0].to_string(),
                qhash.0.elements[1].to_string(),
                qhash.0.elements[2].to_string(),
                qhash.0.elements[3].to_string(),
            ],
        }
    }
}