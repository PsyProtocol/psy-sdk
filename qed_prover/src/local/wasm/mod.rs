use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_core::data::u8bytes::U8Bytes;
use psy_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

use crate::local::store::UserProverWorkerStore;
use crate::local::{args::ContractCallArgs, provider::RpcConfig};
use crate::session::WalletKeyPair;
use crate::session::WalletSession;
use crate::local::args::JobInfo;
use crate::local::args::SignType;
use clap::ValueEnum;

// pub mod wallet_session;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

#[wasm_bindgen]
pub struct WasmRpcServer {
    store: UserProverWorkerStore,
    wallet_session: WalletSession,
}

#[wasm_bindgen]
impl WasmRpcServer {
    #[wasm_bindgen(constructor)]
    pub async fn new(rpc_config_json: &str) -> Result<WasmRpcServer, JsError> {
        let rpc_config: RpcConfig = serde_json::from_str(rpc_config_json)
            .map_err(|e| JsError::new(&format!("Parse RPC config error: {}", e)))?;

        let wallet_session = WalletSession::new(&rpc_config)
        .await
            .map_err(|e| JsError::new(&format!("Create wallet session error: {}", e)))?;

        Ok(WasmRpcServer {
            store: UserProverWorkerStore::new(),
            wallet_session,
        })
    }

    #[wasm_bindgen]
    pub async fn exec_contract_call_json(&mut self, pk_hash: &str, contract_calls_json: &str) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_calls_json)
            .map_err(|e| JsError::new(&format!("Parse exec calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let end_user_leaf_hash = self.wallet_session.exec_contract_call(pk_hash, contract_call_args)
            .await
            .map_err(|e| JsError::new(&format!("Error exec calls error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn exec_contract_call_with_sign_data_json(
        &mut self,
        pk_hash: &str,
        contract_calls_json: &str,
        sign_data: Option<String>,
    ) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_calls_json)
            .map_err(|e| JsError::new(&format!("Parse exec calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let sign_data = sign_data
            .map(|s| serde_json::from_str(&s).map_err(|e| JsError::new(&format!("Parse sign data error: {}", e))))
            .transpose()?;

        let end_user_leaf_hash = self
            .wallet_session
            .exec_contract_call_with_sign_data(pk_hash, contract_call_args, sign_data)
            .await
            .map_err(|e| JsError::new(&format!("Error exec calls error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn get_claim_rewards_call_args_json(
        &self,
        job_infos_json: &str,
    ) -> Result<String, JsError> {
        let job_infos: Vec<JobInfo> = serde_json::from_str(job_infos_json)
            .map_err(|e| JsError::new(&format!("Parse job infos JSON error: {}", e)))?;

        let contract_call_args = self.wallet_session.get_claim_rewards_call_args(job_infos)
            .await
            .map_err(|e| JsError::new(&format!("Error get claim rewards call args error: {}", e)))?;
        Ok(serde_json::to_string(&contract_call_args)?)
    }

    #[wasm_bindgen]
    pub async fn claim_rewards_json(
        &mut self, 
        pk_hash: &str, 
        job_infos_json: &str,
    ) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let job_infos: Vec<JobInfo> = serde_json::from_str(job_infos_json)
            .map_err(|e| JsError::new(&format!("Parse job infos JSON error: {}", e)))?;

        self.wallet_session.claim_rewards(pk_hash, job_infos)
            .await
            .map_err(|e| JsError::new(&format!("Error exec calls error: {}", e)))?;
        Ok("claim_rewards".to_string())
    }

    // Local proving operations
    #[wasm_bindgen]
    pub async fn start_session(&self, pk_hash: &str) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;
        self.wallet_session.start_session(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("Start session error: {}", e)))?;
        Ok("start session".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_call_json(&mut self,pk_hash: &str, contract_call_json: &str) -> Result<String, JsError> {
        let contract_call_arg: ContractCallArgs = serde_json::from_str(contract_call_json)
            .map_err(|e| JsError::new(&format!("Parse contract call JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session.prove_contract_call(pk_hash, contract_call_arg)
            .await
            .map_err(|e| JsError::new(&format!("Prove contract call error: {}", e)))?;
        Ok("prove contract call".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_calls_json(&mut self, pk_hash: &str, contract_calls_json: &str) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_calls_json)
            .map_err(|e| JsError::new(&format!("Parse contract calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session.prove_contract_calls(pk_hash, contract_call_args)
            .await
            .map_err(|e| JsError::new(&format!("Prove contract calls error: {}", e)))?;
        Ok("prove contract calls".to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit(&self, pk_hash: &str) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;
        let end_user_leaf_hash = self.wallet_session.sign_and_submit(pk_hash)
            .await
            .map_err(|e| JsError::new(&format!("Sign and submit error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit_with_sign_data(
        &mut self,
        pk_hash: &str,
        sign_data: Option<String>,
    ) -> Result<String, JsError> {
        let sign_data = sign_data
            .map(|s| serde_json::from_str(&s).map_err(|e| JsError::new(&format!("Parse sign data error: {}", e))))
            .transpose()?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        let end_user_leaf_hash = self
            .wallet_session
            .sign_and_submit_with_sign_data(pk_hash, sign_data)
            .await
            .map_err(|e| JsError::new(&format!("Sign and submit with sign data error: {}", e)))?;
        Ok(end_user_leaf_hash.to_string())
    }

    // User operations
    #[wasm_bindgen]
    pub async fn register_user(&mut self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
        let pk_hash = self.wallet_session.register_user(private_key)
            .await
            .map_err(|e| JsError::new(&format!("Register user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn register_user_with_type(
        &mut self,
        private_key: &str,
        sign_type: &str,
        fingerprint: Option<String>,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
        let sign_type = SignType::from_str(sign_type, true)
            .map_err(|e| JsError::new(&format!("Parse sign type error: {}", e)))?;

        let fingerprint = fingerprint
            .map(|f| QHashOut::<F>::from_str(&f).map_err(|e| JsError::new(&format!("Parse fingerprint error: {}", e))))
            .transpose()?;

        let pk_hash = self.wallet_session.register_user_with_type(private_key, sign_type, fingerprint)
            .await
            .map_err(|e| JsError::new(&format!("Register user with type error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user(&mut self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
        let pk_hash = self.wallet_session.add_user(private_key)
            .await
            .map_err(|e| JsError::new(&format!("Add user error: {}", e)))?;
        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user_with_type(
        &mut self,
        private_key_str: &str,
        sign_type: &str,
        fingerprint: Option<String>,
    ) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
                
        let sign_type = SignType::from_str(sign_type, true)
            .map_err(|e| JsError::new(&format!("Parse sign type error: {}", e)))?;

        let fingerprint = fingerprint
            .map(|f| QHashOut::<F>::from_str(&f).map_err(|e| JsError::new(&format!("Parse fingerprint error: {}", e))))
            .transpose()?;

        let pk_hash = self.wallet_session.add_user_with_type(private_key, sign_type, fingerprint)
            .await
            .map_err(|e| JsError::new(&format!("Add user with sign type error: {}", e)))?;

        Ok(pk_hash.to_string())
    }

    #[wasm_bindgen]
    pub async fn get_zk_public_key_json(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
        let public_key = self.wallet_session.get_zk_public_key(private_key)
            .await
            .map_err(|e| JsError::new(&format!("Get ZK public key error: {}", e)))?;
        serde_json::to_string(&public_key)
            .map_err(|e| JsError::new(&format!("Serialize public key error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn get_random_keypair_json(&self) -> Result<String, JsError> {
        let keypair = self.wallet_session.get_random_keypair()
            .await
            .map_err(|e| JsError::new(&format!("Get random keypair error: {}", e)))?;
        serde_json::to_string(&keypair)
            .map_err(|e| JsError::new(&format!("Serialize keypair error: {}", e)))
    }

    // Contract operations
    #[wasm_bindgen]
    pub async fn deploy_contract_json(&self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsError> {
        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit defs JSON error: {}", e)))?;
        
        self.wallet_session.deploy_contract(deployer, circuit_defs)
            .await
            .map_err(|e| JsError::new(&format!("Deploy contract error: {}", e)))?;
        Ok("deploy contract".to_string())
    }

    #[wasm_bindgen]
    pub fn get_deploy_contract_cmd_json(&self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsError> {
        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit defs JSON error: {}", e)))?;
        
        let cmd = self.wallet_session.get_deploy_contract_cmd(deployer, circuit_defs)
            .map_err(|e| JsError::new(&format!("Get deploy contract cmd error: {}", e)))?;
        serde_json::to_string(&cmd)
            .map_err(|e| JsError::new(&format!("Serialize deploy contract cmd error: {}", e)))
    }

    // Test function
    #[wasm_bindgen]
    pub fn ping(&self, message: &str) -> Result<String, JsError> {
        Ok(format!("Pong: {}", message))
    }

    #[wasm_bindgen]
    pub fn get_result(&self, id_str: &str) -> Result<Vec<u8>, JsError> {
        let id = Hash256::from_hex_string(id_str)
            .map_err(|e| JsError::new(&format!("Invalid ID format: {:?}", e)))?;
        
        match self.store.get_result(&id) {
            Some(proof) => Ok(proof.clone()),
            None => Err(JsError::new(&format!("Proof not found for ID: {}", id_str))),
        }
    }
}