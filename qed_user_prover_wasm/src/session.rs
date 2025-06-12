//! WASM-compatible wallet session implementation

use crate::error::{WasmError, WasmResult};
use crate::types::*;
use anyhow::Result;
use plonky2::field::extension::Extendable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use qed_core::data::qhashout::QHashOut;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Type aliases for consistency
type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// WASM-compatible wallet session that wraps the native WalletSession
#[wasm_bindgen]
pub struct WasmWalletSession {
    session_id: String,
    created_at: u64,
    last_activity: u64,
}

impl WasmWalletSession {
    /// Create a new WASM wallet session
    pub fn new() -> Self {
        Self {
            session_id: crate::utils::generate_session_id(),
            created_at: crate::utils::current_timestamp(),
            last_activity: crate::utils::current_timestamp(),
        }
    }

    /// Create a WASM wallet session from RPC config
    pub fn from_config(_config: &RpcConfig) -> WasmResult<Self> {
        Ok(Self {
            session_id: crate::utils::generate_session_id(),
            created_at: crate::utils::current_timestamp(),
            last_activity: crate::utils::current_timestamp(),
        })
    }

    /// Update last activity timestamp
    fn update_activity(&mut self) {
        self.last_activity = crate::utils::current_timestamp();
    }

    /// Get session information
    pub fn get_session_info(&self) -> SessionInfo {
        SessionInfo {
            session_id: self.session_id.clone(),
            user_id: Some("0".to_string()),
            created_at: self.created_at,
            last_activity: self.last_activity,
        }
    }

    /// Start a new proving session
    pub fn start_session(&mut self) -> WasmResult<String> {
        self.update_activity();
        // For WASM, we just return the session ID
        Ok(self.session_id.clone())
    }

    /// Prove a single contract call
    pub fn prove_contract_call(&mut self, _args: ContractCallArgs) -> WasmResult<ProofWithPublicInputs> {
        self.update_activity();


        // Create a dummy proof for now
        Ok(ProofWithPublicInputs {
            proof: Proof {
                wires_cap: vec![],
                plonk_zs_partial_products_cap: vec![],
                quotient_polys_cap: vec![],
                openings: vec![],
                opening_proof: vec![],
            },
            public_inputs: vec![],
        })
    }

    /// Prove multiple contract calls
    pub fn prove_contract_calls(&mut self, args_list: Vec<ContractCallArgs>) -> WasmResult<Vec<ProofWithPublicInputs>> {
        self.update_activity();
        
        let mut proofs = Vec::new();
        for args in args_list {
            let proof = self.prove_contract_call(args)?;
            proofs.push(proof);
        }
        
        Ok(proofs)
    }

    /// Sign and submit the current session
    pub fn sign_and_submit(&mut self) -> WasmResult<String> {
        self.update_activity();
        
        // For WASM, we simulate the signing and submission
        // In a real implementation, this would interact with the blockchain
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut hash_bytes = [0u8; 32];
        rng.fill_bytes(&mut hash_bytes);
        let transaction_hash = format!("0x{}", hex::encode(hash_bytes));
        
        Ok(transaction_hash)
    }

    /// Register a new user
    pub fn register_user(&mut self, _private_key: QHashOut<F>) -> WasmResult<QHashOut<F>> {
        self.update_activity();
        
        
        Ok(QHashOut::rand())
    }

    /// Add an existing user
    pub fn add_user(&mut self, _private_key: QHashOut<F>) -> WasmResult<QHashOut<F>> {
        self.update_activity();


        Ok(QHashOut::rand())
    }

    /// Switch to a different user
    pub fn switch_user(&mut self, _pk_hash: QHashOut<F>) -> WasmResult<()> {
        self.update_activity();

        Ok(())
    }

    /// Get ZK public key for a private key
    pub fn get_zk_public_key(&self, private_key: QHashOut<F>) -> WasmResult<String> {
        // For WASM, we simulate the ZK public key generation
        // In a real implementation, this would use the actual cryptographic functions
        let bytes: Vec<u8> = private_key.0.elements.iter().flat_map(|f| f.0.to_le_bytes()).collect();
        let public_key = format!("zk_pk_{}", hex::encode(bytes));
        Ok(public_key)
    }

    /// Generate a random keypair
    pub fn get_random_keypair(&self) -> WasmResult<KeypairInfo> {
        let private_key = QHashOut::<F>::rand();
        let public_key = self.get_zk_public_key(private_key)?;
        
        Ok(KeypairInfo {
            private_key: serde_json::to_string(&private_key)?,
            public_key,
        })
    }

    /// Deploy a contract
    pub fn deploy_contract(&mut self, circuit_defs: Vec<DPNFunctionCircuitDefinition>) -> WasmResult<()> {
        self.update_activity();
        
        // For WASM, we simulate contract deployment
        // In a real implementation, this would compile and deploy the contract
        crate::utils::log(&format!("Deploying contract with {} functions", circuit_defs.len()));
        
        Ok(())
    }

    /// Get deploy contract command
    pub fn get_deploy_contract_cmd(&mut self, circuit_defs: Vec<DPNFunctionCircuitDefinition>) -> WasmResult<DeployContractCmd> {
        self.update_activity();
        
        // Generate a deployment command
        let contract_code = serde_json::to_string(&circuit_defs)?;
        
        Ok(DeployContractCmd {
            contract_code,
            constructor_args: vec![],
            gas_limit: 1000000,
        })
    }

    /// Get signature hash
    pub fn get_sig_hash(&self, network_magic: u64) -> WasmResult<QHashOut<F>> {
        // Generate a signature hash based on network magic and current state
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hasher::write_u64(&mut hasher, network_magic);
        std::hash::Hasher::write_u64(&mut hasher, self.last_activity);
        
        let hash_value = std::hash::Hasher::finish(&hasher);
        let elements = [
            F::from_canonical_u64(hash_value),
            F::from_canonical_u64(hash_value >> 16),
            F::from_canonical_u64(hash_value >> 32),
            F::from_canonical_u64(hash_value >> 48),
        ];
        
        Ok(QHashOut(plonky2::hash::hash_types::HashOut { elements }))
    }

    /// Get ZK signature
    pub fn get_zk_signature(&self, sighash: QHashOut<F>) -> WasmResult<ZKSignature> {
        // For WASM, we simulate ZK signature generation
        let bytes: Vec<u8> = sighash.0.elements.iter().flat_map(|f| f.0.to_le_bytes()).collect();
        let signature = format!("zk_sig_{}", hex::encode(&bytes));
        let public_key = format!("zk_pk_{}", hex::encode(&bytes));
        let message_hash = hex::encode(&bytes);
        
        Ok(ZKSignature {
            signature,
            public_key,
            message_hash,
        })
    }

    /// Get end cap proof
    pub fn get_end_cap_proof(&mut self, signature_proof: crate::types::ProofWithPublicInputs) -> WasmResult<ProofWithPublicInputs> {
        self.update_activity();
        
        // Convert the native proof to WASM format
        self.convert_proof_to_wasm(signature_proof)
    }

    /// Get user end cap input
    pub fn get_user_ec_input(&mut self) -> WasmResult<SubmitUserEndCapNonProofInput> {
        self.update_activity();
        
        // Generate user end cap input
        let core_input = SubmitUserEndCapNonProofCoreInput {
            user_id: "0".to_string(),
            contract_address: "0x1234567890abcdef".to_string(),
            function_name: "default_function".to_string(),
            inputs: vec![],
            timestamp: self.last_activity,
        };
        
        let state_history = QEDContractStateUpdateHistory {
            contract_address: "0x1234567890abcdef".to_string(),
            updates: vec![],
        };
        
        Ok(SubmitUserEndCapNonProofInput {
            core_input,
            state_history,
            additional_data: None,
        })
    }
    
    fn convert_proof_to_wasm(&self, _proof: crate::types::ProofWithPublicInputs) -> WasmResult<ProofWithPublicInputs> {
        // Create a dummy proof for now
        Ok(ProofWithPublicInputs {
            proof: Proof {
                wires_cap: vec![],
                plonk_zs_partial_products_cap: vec![],
                quotient_polys_cap: vec![],
                openings: vec![], // Changed to Vec<String>
                opening_proof: vec![], // Changed from FriProof to Vec<String>
            },
            public_inputs: vec![],
        })
    }
}

impl Default for WasmWalletSession {
    fn default() -> Self {
        Self::new()
    }
}