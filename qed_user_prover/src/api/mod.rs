cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use jsonrpsee::core::async_trait;
        use jsonrpsee::proc_macros::rpc;
        use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};
    }
}

use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_core::data::u8bytes::U8Bytes;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_user_cli::subcommand::args::ContractCallArgs;
use qed_user_cli::session::{WalletKeyPair, WalletSession};
use qed_user_cli::rpc::provider::RpcConfig;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use crate::store::UserProverWorkerStore;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

// Only define RPC trait for non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
#[rpc(server, client, namespace = "qed")]
pub trait Rpc {
    /// local proving operation
    #[method(name = "exec_contract_call")]
    async fn exec_contract_call(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "start_session")]
    async fn start_session(&self, pk_hash: QHashOut<F>) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_call")]
    async fn prove_contract_call(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_arg: ContractCallArgs,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_calls")]
    async fn prove_contract_calls(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned>;

    #[method(name = "sign_and_submit")]
    async fn sign_and_submit(&self, pk_hash: QHashOut<F>) -> Result<String, ErrorObjectOwned>;

    /// user operation
    #[method(name = "register_user")]
    async fn register_user(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<QHashOut<F>, ErrorObjectOwned>;
    #[method(name = "add_user")]
    async fn add_user(&self, private_key: QHashOut<F>) -> Result<QHashOut<F>, ErrorObjectOwned>;
    // #[method(name = "switch_user")]
    // async fn switch_user(&self, pk_hash: QHashOut<F>) -> Result<(), ErrorObjectOwned>;
    #[method(name = "get_zk_public_key")]
    async fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<ZKPublicKeyInfo<F>, ErrorObjectOwned>;
    #[method(name = "get_random_keypair")]
    async fn get_random_keypair(&self) -> Result<WalletKeyPair, ErrorObjectOwned>;

    /// deploy contract
    #[method(name = "deploy_contract")]
    async fn deploy_contract(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "get_deploy_contract_cmd")]
    async fn get_deploy_contract_cmd(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<GoldilocksField>, ErrorObjectOwned>;

    /// sign and submit
    // #[method(name = "get_sighash")]
    // async fn get_sighash(&self, network_magic: u64) -> Result<QHashOut<F>, ErrorObjectOwned>;
    // #[method(name = "get_zk_signature")]
    // async fn get_zk_signature(
    //     &self,
    //     sighash: QHashOut<F>,
    // ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    // #[method(name = "get_end_cap_proof")]
    // async fn get_end_cap_proof(
    //     &self,
    //     signature_proof: ProofWithPublicInputs<F, C, D>,
    // ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    // #[method(name = "get_user_ec_input")]
    // async fn get_user_ec_input(&self)
    //     -> Result<SubmitUserEndCapNonProofInput<F>, ErrorObjectOwned>;

    #[method(name = "ping")]
    async fn ping(&self, message: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "get_result")]
    async fn get_result(&self, id: Hash256) -> Result<U8Bytes, ErrorObjectOwned>;
}

// For non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
pub struct RpcServerImpl {
    pub store: Arc<Mutex<UserProverWorkerStore>>,
    pub wallet_session: Arc<RwLock<WalletSession>>,
}

// Create a WASM-compatible wrapper struct
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmRpcServer {
    store: UserProverWorkerStore,
    wallet_session: WalletSession,
}


#[maybe_async::maybe_async(?Send)]
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmRpcServer {
    #[wasm_bindgen(constructor)]
    pub fn new(rpc_config_json: &str) -> Result<WasmRpcServer, JsError> {
        let rpc_config: RpcConfig = serde_json::from_str(rpc_config_json)
            .map_err(|e| JsError::new(&format!("Parse RPC config error: {}", e)))?;

        let wallet_session = WalletSession::new(&rpc_config)
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

        self.wallet_session.exec_contract_call(pk_hash, contract_call_args).await
            .map_err(|e| JsError::new(&format!("Error exec calls error: {}", e)))?;
        Ok("start session".to_string())
    }

    // Local proving operations
    #[wasm_bindgen]
    pub async fn start_session(&self, pk_hash: &str) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;
        self.wallet_session.start_session(pk_hash).await
            .map_err(|e| JsError::new(&format!("Start session error: {}", e)))?;
        Ok("start session".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_call_json(&mut self,pk_hash: &str, contract_call_json: &str) -> Result<String, JsError> {
        let contract_call_arg: ContractCallArgs = serde_json::from_str(contract_call_json)
            .map_err(|e| JsError::new(&format!("Parse contract call JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session.prove_contract_call(pk_hash, contract_call_arg).await
            .map_err(|e| JsError::new(&format!("Prove contract call error: {}", e)))?;
        Ok("prove contract call".to_string())
    }

    #[wasm_bindgen]
    pub async fn prove_contract_calls_json(&mut self, pk_hash: &str, contract_calls_json: &str) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_calls_json)
            .map_err(|e| JsError::new(&format!("Parse contract calls JSON error: {}", e)))?;

        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session.prove_contract_calls(pk_hash, contract_call_args).await
            .map_err(|e| JsError::new(&format!("Prove contract calls error: {}", e)))?;
        Ok("prove contract calls".to_string())
    }

    #[wasm_bindgen]
    pub async fn sign_and_submit(&mut self, pk_hash: &str) -> Result<String, JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;

        self.wallet_session.sign_and_submit(pk_hash).await
            .map_err(|e| JsError::new(&format!("Sign and submit error: {}", e)))?;
        Ok("sign and submit".to_string())
    }

    // User operations
    #[wasm_bindgen]
    pub async fn register_user(&mut self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let result = self.wallet_session.register_user(private_key).await
            .map_err(|e| JsError::new(&format!("Register user error: {}", e)))?;

        Ok(result.to_string())
    }

    #[wasm_bindgen]
    pub async fn add_user(&mut self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let result = self.wallet_session.add_user(private_key).await
            .map_err(|e| JsError::new(&format!("Add user error: {}", e)))?;

        Ok(result.to_string())
    }

    #[wasm_bindgen]
    pub async fn get_zk_public_key_json(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;

        let result = self.wallet_session.get_zk_public_key(private_key)
            .map_err(|e| JsError::new(&format!("Get ZK public key error: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize ZK public key error: {}", e)))
    }

    #[wasm_bindgen]
    pub async fn get_random_keypair_json(&self) -> Result<String, JsError> {
        let result = self.wallet_session.get_random_keypair()
            .map_err(|e| JsError::new(&format!("Get random keypair error: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize keypair error: {}", e)))
    }

    // Contract operations
    #[wasm_bindgen]
    pub async fn deploy_contract_json(&mut self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsError> {
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit definitions JSON error: {}", e)))?;

        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;

        self.wallet_session.deploy_contract(deployer, circuit_defs).await
            .map_err(|e| JsError::new(&format!("Deploy contract error: {}", e)))?;
        Ok("deploy contract".to_string())
    }

    #[wasm_bindgen]
    pub async fn get_deploy_contract_cmd_json(&self, deployer: &str, circuit_defs_json: &str) -> Result<String, JsError> {
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit definitions JSON error: {}", e)))?;

        let deployer = QHashOut::<F>::from_str(deployer)
            .map_err(|e| JsError::new(&format!("Parse deployer error: {}", e)))?;

        let result = self.wallet_session.get_deploy_contract_cmd(deployer, circuit_defs)
            .map_err(|e| JsError::new(&format!("Get deploy contract cmd error: {}", e)))?;

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize deploy contract cmd error: {}", e)))
    }

    // Utility operations
    #[wasm_bindgen]
    pub async fn ping(&self, message: String) -> Result<String, JsError> {
        let response = message.chars().rev().collect::<String>();
        Ok(response)
    }

    #[wasm_bindgen]
    pub async fn get_result(&mut self, id_str: &str) -> Result<Vec<u8>, JsError> {
        let id = Hash256::try_from(id_str)
            .map_err(|e| JsError::new(&format!("Parse hash ID error: {}", e)))?;

        let result = self.store.get_result_and_clear(&id);

        if result.is_none() {
            return Err(JsError::new("Result not found"));
        }
        Ok(result.unwrap())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RpcServerImpl {
    pub fn new(
        store: Arc<Mutex<UserProverWorkerStore>>,
        wallet_session: Arc<RwLock<WalletSession>>,
    ) -> Self {
        Self {
            store,
            wallet_session,
        }
    }

    pub fn write_wallet(&self) -> anyhow::Result<RwLockWriteGuard<WalletSession>> {
        self.wallet_session
            .try_write()
            .map_err(|err| anyhow::anyhow!("Error writing to immutable store: {:?}", err))
    }
    pub fn read_wallet(&self) -> anyhow::Result<RwLockReadGuard<WalletSession>> {
        self.wallet_session
            .try_read()
            .map_err(|err| anyhow::anyhow!("Error reading from immutable store: {:?}", err))
    }


}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn exec_contract_call(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned> {
        tracing::info!(
            "exec_contract_call with `{:?}`: {:?}",
            pk_hash.to_string(),
            serde_json::to_string_pretty(&contract_call_args).unwrap()
        );
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .exec_contract_call(pk_hash, contract_call_args)
            .map_err(|e| {
                ErrorObject::owned(600, "Error exec contract call", Some(e.to_string()))
            })?;
        Ok("start session".to_string())
    }
    async fn start_session(&self, pk_hash: QHashOut<F>) -> Result<String, ErrorObjectOwned> {
        tracing::info!("start_session with `{:?}`", pk_hash.to_string());
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .start_session(pk_hash)
            .map_err(|e| ErrorObject::owned(601, "Error starting session", Some(e.to_string())))?;
        Ok("start session".to_string())
    }
    async fn prove_contract_call(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_arg: ContractCallArgs,
    ) -> Result<String, ErrorObjectOwned> {
        tracing::info!(
            "prove_contract_call with `{:?}`: {:?}",
            pk_hash.to_string(),
            serde_json::to_string_pretty(&contract_call_arg).unwrap()
        );
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .prove_contract_call(pk_hash, contract_call_arg)
            .map_err(|e| {
                ErrorObject::owned(602, "Error prove contract call", Some(e.to_string()))
            })?;
        Ok("prove contract call".to_string())
    }
    async fn prove_contract_calls(
        &self,
        pk_hash: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned> {
        tracing::info!(
            "prove_contract_calls with `{:?}`: {:?}",
            pk_hash.to_string(),
            serde_json::to_string_pretty(&contract_call_args).unwrap()
        );
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .prove_contract_calls(pk_hash, contract_call_args)
            .map_err(|e| {
                ErrorObject::owned(603, "Error prove contract calls", Some(e.to_string()))
            })?;
        Ok("prove contract calls".to_string())
    }

    async fn sign_and_submit(&self, pk_hash: QHashOut<F>) -> Result<String, ErrorObjectOwned> {
        tracing::info!("sign_and_submit with `{:?}`", pk_hash.to_string());
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .sign_and_submit(pk_hash)
            .map_err(|e| ErrorObject::owned(604, "Error sign and submit", Some(e.to_string())))?;
        Ok("sign and submit".to_string())
    }

    async fn register_user(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<QHashOut<F>, ErrorObjectOwned> {
        tracing::info!("register_user with `{:?}`", private_key.to_string());
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .register_user(private_key)
            .map_err(|e| ErrorObject::owned(605, "Error sign and submit", Some(e.to_string())))
    }

    async fn add_user(&self, private_key: QHashOut<F>) -> Result<QHashOut<F>, ErrorObjectOwned> {
        tracing::info!("register_user with `{:?}`", private_key.to_string());
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .add_user(private_key)
            .map_err(|e| ErrorObject::owned(606, "Error add user", Some(e.to_string())))
    }

    async fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<ZKPublicKeyInfo<F>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_zk_public_key(private_key)
            .map_err(|e| ErrorObject::owned(608, "Error get zk public key", Some(e.to_string())))
    }

    async fn get_random_keypair(&self) -> Result<WalletKeyPair, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_random_keypair()
            .map_err(|e| ErrorObject::owned(609, "Error get random keypair", Some(e.to_string())))
    }

    async fn deploy_contract(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .deploy_contract(deployer, circuit_defs)
            .map_err(|e| ErrorObject::owned(610, "Error deploy contract", Some(e.to_string())))?;
        Ok("deploy contract".to_string())
    }

    async fn get_deploy_contract_cmd(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<F>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_deploy_contract_cmd(deployer, circuit_defs)
            .map_err(|e| {
                ErrorObject::owned(611, "Error get deploy contract cmd", Some(e.to_string()))
            })
    }

    async fn ping(&self, message: String) -> Result<String, ErrorObjectOwned> {
        Ok(message.chars().rev().collect::<String>())
    }
    async fn get_result(&self, id: Hash256) -> Result<U8Bytes, ErrorObjectOwned> {
        let result = self.store.lock().unwrap().get_result_and_clear(&id);

        if result.is_none() {
            return Err(ErrorObject::owned(404, "Result not found", Some(0)));
        }
        Ok(U8Bytes(result.unwrap()))
    }
}
