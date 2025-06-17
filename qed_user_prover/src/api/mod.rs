#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::core::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::proc_macros::rpc;
#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};

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
    #[method(name = "start_session")]
    fn start_session(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_call")]
    fn prove_contract_call(
        &self,
        contract_call_arg: ContractCallArgs,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_calls")]
    fn prove_contract_calls(
        &self,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned>;

    #[method(name = "sign_and_submit")]
    fn sign_and_submit(&self) -> Result<String, ErrorObjectOwned>;

    /// user operation
    #[method(name = "register_user")]
    fn register_user(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<QHashOut<F>, ErrorObjectOwned>;
    #[method(name = "add_user")]
    fn add_user(&self, private_key: QHashOut<F>) -> Result<QHashOut<F>, ErrorObjectOwned>;
    #[method(name = "switch_user")]
    fn switch_user(&self, pk_hash: QHashOut<F>) -> Result<(), ErrorObjectOwned>;
    #[method(name = "get_zk_public_key")]
    fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<ZKPublicKeyInfo<F>, ErrorObjectOwned>;
    #[method(name = "get_random_keypair")]
    fn get_random_keypair(&self) -> Result<WalletKeyPair, ErrorObjectOwned>;

    /// deploy contract
    #[method(name = "deploy_contract")]
    fn deploy_contract(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "get_deploy_contract_cmd")]
    fn get_deploy_contract_cmd(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<GoldilocksField>, ErrorObjectOwned>;

    /// sign and submit
    #[method(name = "get_sighash")]
    fn get_sighash(&self, network_magic: u64) -> Result<QHashOut<F>, ErrorObjectOwned>;
    #[method(name = "get_zk_signature")]
    fn get_zk_signature(
        &self,
        sighash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    #[method(name = "get_end_cap_proof")]
    fn get_end_cap_proof(
        &self,
        signature_proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    #[method(name = "get_user_ec_input")]
    fn get_user_ec_input(&self)
        -> Result<SubmitUserEndCapNonProofInput<F>, ErrorObjectOwned>;

    #[method(name = "ping")]
    fn ping(&self, message: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "get_result")]
    fn get_result(&self, id: Hash256) -> Result<U8Bytes, ErrorObjectOwned>;
}

pub struct RpcServerImpl {
    pub store: Arc<Mutex<UserProverWorkerStore>>,
    pub wallet_session: Arc<RwLock<WalletSession>>,
}

// Create a WASM-compatible wrapper struct
#[wasm_bindgen]
pub struct WasmRpcServer {
    inner: RpcServerImpl,
}

#[wasm_bindgen]
impl WasmRpcServer {
    #[wasm_bindgen(constructor)]
    pub fn new(rpc_config_json: &str) -> Result<WasmRpcServer, JsError> {
        console_error_panic_hook::set_once();
        
        let rpc_config: RpcConfig = serde_json::from_str(rpc_config_json)
            .map_err(|e| JsError::new(&format!("Parse RPC config error: {}", e)))?;
            
        let wallet_session = WalletSession::new(&rpc_config)
            .map_err(|e| JsError::new(&format!("Create wallet session error: {}", e)))?;
        
        let store = Arc::new(Mutex::new(UserProverWorkerStore::new()));
        let wallet_session = Arc::new(RwLock::new(wallet_session));
        
        let inner = RpcServerImpl::new(store, wallet_session);
        
        Ok(WasmRpcServer { inner })
    }

    // Local proving operations
    #[wasm_bindgen]
    pub fn start_session(&self) -> Result<String, JsError> {
        self.inner.start_session_internal()
            .map_err(|e| JsError::new(&format!("Start session error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn prove_contract_call_json(&self, contract_call_json: &str) -> Result<String, JsError> {
        let contract_call_arg: ContractCallArgs = serde_json::from_str(contract_call_json)
            .map_err(|e| JsError::new(&format!("Parse contract call JSON error: {}", e)))?;
            
        self.inner.prove_contract_call_internal(contract_call_arg)
            .map_err(|e| JsError::new(&format!("Prove contract call error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn prove_contract_calls_json(&self, contract_calls_json: &str) -> Result<String, JsError> {
        let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(contract_calls_json)
            .map_err(|e| JsError::new(&format!("Parse contract calls JSON error: {}", e)))?;
            
        self.inner.prove_contract_calls_internal(contract_call_args)
            .map_err(|e| JsError::new(&format!("Prove contract calls error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn sign_and_submit(&self) -> Result<String, JsError> {
        self.inner.sign_and_submit_internal()
            .map_err(|e| JsError::new(&format!("Sign and submit error: {}", e)))
    }

    // User operations
    #[wasm_bindgen]
    pub fn register_user(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
            
        let result = self.inner.register_user_internal(private_key)
            .map_err(|e| JsError::new(&format!("Register user error: {}", e)))?;
            
        Ok(result.to_string())
    }

    #[wasm_bindgen]
    pub fn add_user(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
            
        let result = self.inner.add_user_internal(private_key)
            .map_err(|e| JsError::new(&format!("Add user error: {}", e)))?;
            
        Ok(result.to_string())
    }

    #[wasm_bindgen]
    pub fn switch_user(&self, pk_hash_str: &str) -> Result<(), JsError> {
        let pk_hash = QHashOut::<F>::from_str(pk_hash_str)
            .map_err(|e| JsError::new(&format!("Parse public key hash error: {}", e)))?;
            
        self.inner.switch_user_internal(pk_hash)
            .map_err(|e| JsError::new(&format!("Switch user error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_zk_public_key_json(&self, private_key_str: &str) -> Result<String, JsError> {
        let private_key = QHashOut::<F>::from_str(private_key_str)
            .map_err(|e| JsError::new(&format!("Parse private key error: {}", e)))?;
            
        let result = self.inner.get_zk_public_key_internal(private_key)
            .map_err(|e| JsError::new(&format!("Get ZK public key error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize ZK public key error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_random_keypair_json(&self) -> Result<String, JsError> {
        let result = self.inner.get_random_keypair_internal()
            .map_err(|e| JsError::new(&format!("Get random keypair error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize keypair error: {}", e)))
    }

    // Contract operations
    #[wasm_bindgen]
    pub fn deploy_contract_json(&self, circuit_defs_json: &str) -> Result<String, JsError> {
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit definitions JSON error: {}", e)))?;
            
        self.inner.deploy_contract_internal(circuit_defs)
            .map_err(|e| JsError::new(&format!("Deploy contract error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_deploy_contract_cmd_json(&self, circuit_defs_json: &str) -> Result<String, JsError> {
        let circuit_defs: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(circuit_defs_json)
            .map_err(|e| JsError::new(&format!("Parse circuit definitions JSON error: {}", e)))?;
            
        let result = self.inner.get_deploy_contract_cmd_internal(circuit_defs)
            .map_err(|e| JsError::new(&format!("Get deploy contract cmd error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize deploy contract cmd error: {}", e)))
    }

    // Signature operations
    #[wasm_bindgen]
    pub fn get_sighash(&self, network_magic: u64) -> Result<String, JsError> {
        let result = self.inner.get_sighash_internal(network_magic)
            .map_err(|e| JsError::new(&format!("Get sighash error: {}", e)))?;
            
        Ok(result.to_string())
    }

    #[wasm_bindgen]
    pub fn get_zk_signature_json(&self, sighash_str: &str) -> Result<String, JsError> {
        let sighash = QHashOut::<F>::from_str(sighash_str)
            .map_err(|e| JsError::new(&format!("Parse sighash error: {}", e)))?;
            
        let result = self.inner.get_zk_signature_internal(sighash)
            .map_err(|e| JsError::new(&format!("Get ZK signature error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize ZK signature error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_end_cap_proof_json(&self, signature_proof_json: &str) -> Result<String, JsError> {
        let signature_proof: ProofWithPublicInputs<F, C, D> = serde_json::from_str(signature_proof_json)
            .map_err(|e| JsError::new(&format!("Parse signature proof JSON error: {}", e)))?;
            
        let result = self.inner.get_end_cap_proof_internal(signature_proof)
            .map_err(|e| JsError::new(&format!("Get end cap proof error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize end cap proof error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_user_ec_input_json(&self) -> Result<String, JsError> {
        let result = self.inner.get_user_ec_input_internal()
            .map_err(|e| JsError::new(&format!("Get user EC input error: {}", e)))?;
            
        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&format!("Serialize user EC input error: {}", e)))
    }

    // Utility operations
    #[wasm_bindgen]
    pub fn ping(&self, message: String) -> Result<String, JsError> {
        self.inner.ping_internal(message)
            .map_err(|e| JsError::new(&format!("Ping error: {}", e)))
    }

    #[wasm_bindgen]
    pub fn get_result(&self, id_str: &str) -> Result<Vec<u8>, JsError> {
        let id = Hash256::try_from(id_str)
            .map_err(|e| JsError::new(&format!("Parse hash ID error: {}", e)))?;
            
        let result = self.inner.get_result_internal(id)
            .map_err(|e| JsError::new(&format!("Get result error: {}", e)))?;
            
        Ok(result.0)
    }
}

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

    // Internal methods for WASM compatibility
    pub fn start_session_internal(&self) -> anyhow::Result<String> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .start_session()?;
        Ok("start session".to_string())
    }

    pub fn prove_contract_call_internal(&self, contract_call_arg: ContractCallArgs) -> anyhow::Result<String> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .prove_contract_call(contract_call_arg)?;
        Ok("prove contract call".to_string())
    }

    pub fn prove_contract_calls_internal(&self, contract_call_args: Vec<ContractCallArgs>) -> anyhow::Result<String> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .prove_contract_calls(contract_call_args)?;
        Ok("prove contract calls".to_string())
    }

    pub fn sign_and_submit_internal(&self) -> anyhow::Result<String> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .sign_and_submit()?;
        Ok("sign and submit".to_string())
    }

    pub fn register_user_internal(&self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .register_user(private_key)
    }

    pub fn add_user_internal(&self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .add_user(private_key)
    }

    pub fn switch_user_internal(&self, pk_hash: QHashOut<F>) -> anyhow::Result<()> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .switch_user(pk_hash)
    }

    pub fn get_zk_public_key_internal(&self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        self.wallet_session
            .read()
            .map_err(|e| anyhow::anyhow!("Error read wallet session: {}", e))?
            .get_zk_public_key(private_key)
    }

    pub fn get_random_keypair_internal(&self) -> anyhow::Result<WalletKeyPair> {
        self.wallet_session
            .read()
            .map_err(|e| anyhow::anyhow!("Error read wallet session: {}", e))?
            .get_random_keypair()
    }

    pub fn deploy_contract_internal(&self, circuit_defs: Vec<DPNFunctionCircuitDefinition>) -> anyhow::Result<String> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .deploy_contract(circuit_defs)?;
        Ok("deploy contract".to_string())
    }

    pub fn get_deploy_contract_cmd_internal(&self, circuit_defs: Vec<DPNFunctionCircuitDefinition>) -> anyhow::Result<QBCDeployContract<F>> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .get_deploy_contract_cmd(circuit_defs)
    }

    pub fn get_sighash_internal(&self, network_magic: u64) -> anyhow::Result<QHashOut<F>> {
        self.wallet_session
            .read()
            .map_err(|e| anyhow::anyhow!("Error read wallet session: {}", e))?
            .get_sig_hash(network_magic)
    }

    pub fn get_zk_signature_internal(&self, sighash: QHashOut<F>) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        self.wallet_session
            .read()
            .map_err(|e| anyhow::anyhow!("Error read wallet session: {}", e))?
            .get_zk_signature(sighash)
    }

    pub fn get_end_cap_proof_internal(&self, signature_proof: ProofWithPublicInputs<F, C, D>) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .get_end_cap_proof(signature_proof)
    }

    pub fn get_user_ec_input_internal(&self) -> anyhow::Result<SubmitUserEndCapNonProofInput<F>> {
        self.wallet_session
            .write()
            .map_err(|e| anyhow::anyhow!("Error write wallet session: {}", e))?
            .get_user_ec_input()
    }

    pub fn ping_internal(&self, message: String) -> anyhow::Result<String> {
        Ok(message.chars().rev().collect::<String>())
    }

    pub fn get_result_internal(&self, id: Hash256) -> anyhow::Result<U8Bytes> {
        let result = self.store.lock().unwrap().get_result_and_clear(&id);
        
        if result.is_none() {
            return Err(anyhow::anyhow!("Result not found"));
        }
        Ok(U8Bytes(result.unwrap()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl RpcServer for RpcServerImpl {
    fn start_session(&self) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .start_session()
            .map_err(|e| ErrorObject::owned(601, "Error starting session", Some(e.to_string())))?;
        Ok("start session".to_string())
    }
    fn prove_contract_call(
        &self,
        contract_call_arg: ContractCallArgs,
    ) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .prove_contract_call(contract_call_arg)
            .map_err(|e| {
                ErrorObject::owned(602, "Error prove contract call", Some(e.to_string()))
            })?;
        Ok("prove contract call".to_string())
    }
    fn prove_contract_calls(
        &self,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .prove_contract_calls(contract_call_args)
            .map_err(|e| {
                ErrorObject::owned(603, "Error prove contract calls", Some(e.to_string()))
            })?;
        Ok("prove contract calls".to_string())
    }

    fn sign_and_submit(&self) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .sign_and_submit()
            .map_err(|e| ErrorObject::owned(604, "Error sign and submit", Some(e.to_string())))?;
        Ok("sign and submit".to_string())
    }

    fn register_user(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<QHashOut<F>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .register_user(private_key)
            .map_err(|e| ErrorObject::owned(605, "Error sign and submit", Some(e.to_string())))
    }

    fn add_user(&self, private_key: QHashOut<F>) -> Result<QHashOut<F>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .add_user(private_key)
            .map_err(|e| ErrorObject::owned(606, "Error add user", Some(e.to_string())))
    }

    fn switch_user(&self, pk_hash: QHashOut<F>) -> Result<(), ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .switch_user(pk_hash)
            .map_err(|e| ErrorObject::owned(607, "Error switch user", Some(e.to_string())))
    }

    fn get_zk_public_key(
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

    fn get_random_keypair(&self) -> Result<WalletKeyPair, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_random_keypair()
            .map_err(|e| ErrorObject::owned(609, "Error get random keypair", Some(e.to_string())))
    }

    fn deploy_contract(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .deploy_contract(circuit_defs)
            .map_err(|e| ErrorObject::owned(610, "Error deploy contract", Some(e.to_string())))?;
        Ok("deploy contract".to_string())
    }

    fn get_deploy_contract_cmd(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<F>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_deploy_contract_cmd(circuit_defs)
            .map_err(|e| {
                ErrorObject::owned(611, "Error get deploy contract cmd", Some(e.to_string()))
            })
    }

    fn get_sighash(&self, network_magic: u64) -> Result<QHashOut<F>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_sig_hash(network_magic)
            .map_err(|e| ErrorObject::owned(612, "Error get sighash", Some(e.to_string())))
    }

    fn get_zk_signature(
        &self,
        sighash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_zk_signature(sighash)
            .map_err(|e| ErrorObject::owned(613, "Error get zk signature", Some(e.to_string())))
    }

    fn get_end_cap_proof(
        &self,
        signature_proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_end_cap_proof(signature_proof)
            .map_err(|e| ErrorObject::owned(614, "Error get end cap proof", Some(e.to_string())))
    }

    fn get_user_ec_input(
        &self,
    ) -> Result<SubmitUserEndCapNonProofInput<F>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_user_ec_input()
            .map_err(|e| ErrorObject::owned(615, "Error get user ec input", Some(e.to_string())))
    }

    fn ping(&self, message: String) -> Result<String, ErrorObjectOwned> {
        Ok(message.chars().rev().collect::<String>())
    }
    fn get_result(&self, id: Hash256) -> Result<U8Bytes, ErrorObjectOwned> {
        let result = self.store.lock().unwrap().get_result_and_clear(&id);

        if result.is_none() {
            return Err(ErrorObject::owned(404, "Result not found", Some(0)));
        }
        Ok(U8Bytes(result.unwrap()))
    }
}
