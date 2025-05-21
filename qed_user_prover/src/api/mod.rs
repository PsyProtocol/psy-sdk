use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
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
use qed_user_cli::subcommand::session::WalletSession;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::store::UserProverWorkerStore;

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;

#[rpc(server, client, namespace = "qed")]
pub trait Rpc {
    /// wallet operation
    #[method(name = "start_session")]
    async fn start_session(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_call")]
    async fn prove_contract_call(
        &self,
        contract_call_arg: ContractCallArgs,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "prove_contract_calls")]
    async fn prove_contract_calls(
        &self,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<String, ErrorObjectOwned>;
    #[method(name = "get_sighash")]
    async fn get_sighash(&self, network_magic: u64) -> Result<QHashOut<F>, ErrorObjectOwned>;
    #[method(name = "get_zk_signature")]
    async fn get_zk_signature(
        &self,
        sighash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    #[method(name = "get_end_cap_proof")]
    async fn get_end_cap_proof(
        &self,
        signature_proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
    #[method(name = "get_user_ec_input")]
    async fn get_user_ec_input(&self)
        -> Result<SubmitUserEndCapNonProofInput<F>, ErrorObjectOwned>;

    /// register user
    #[method(name = "get_zk_public_key")]
    async fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> Result<ZKPublicKeyInfo<F>, ErrorObjectOwned>;

    /// deploy contract
    #[method(name = "get_deploy_contract_cmd")]
    async fn get_deploy_contract_cmd(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> Result<QBCDeployContract<GoldilocksField>, ErrorObjectOwned>;

    #[method(name = "ping")]
    async fn ping(&self, message: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "get_result")]
    async fn get_result(&self, id: Hash256) -> Result<U8Bytes, ErrorObjectOwned>;
}

pub struct RpcServerImpl {
    pub store: Arc<Mutex<UserProverWorkerStore>>,
    pub wallet_session: Arc<RwLock<WalletSession>>,
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
}

#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn start_session(&self) -> Result<String, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .start_session()
            .map_err(|e| ErrorObject::owned(601, "Error starting session", Some(e.to_string())))?;
        Ok("start session".to_string())
    }
    async fn prove_contract_call(
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
    async fn prove_contract_calls(
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

    async fn get_sighash(&self, network_magic: u64) -> Result<QHashOut<F>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_sig_hash(network_magic)
            .map_err(|e| ErrorObject::owned(604, "Error get sighash", Some(e.to_string())))
    }

    async fn get_zk_signature(
        &self,
        sighash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        self.wallet_session
            .read()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_zk_signature(sighash)
            .map_err(|e| ErrorObject::owned(605, "Error get zk signature", Some(e.to_string())))
    }

    async fn get_end_cap_proof(
        &self,
        signature_proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_end_cap_proof(signature_proof)
            .map_err(|e| ErrorObject::owned(606, "Error get end cap proof", Some(e.to_string())))
    }

    async fn get_user_ec_input(
        &self,
    ) -> Result<SubmitUserEndCapNonProofInput<F>, ErrorObjectOwned> {
        self.wallet_session
            .write()
            .map_err(|e| {
                ErrorObject::owned(500, "Error write wallet session", Some(e.to_string()))
            })?
            .get_user_ec_input()
            .map_err(|e| ErrorObject::owned(607, "Error get user ec input", Some(e.to_string())))
    }

    async fn get_deploy_contract_cmd(
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
                ErrorObject::owned(608, "Error get deploy contract cmd", Some(e.to_string()))
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
            .map_err(|e| ErrorObject::owned(609, "Error get zk public key", Some(e.to_string())))
    }
}
