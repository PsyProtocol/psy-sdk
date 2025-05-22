use std::str::FromStr;

use anyhow::Ok;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::poseidon::PoseidonHash,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{
        GLOBAL_USER_TREE_HEIGHT, QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT,
    },
    data::qhashout::QHashOut,
    ups::circuits::LocalCircuitType,
};
use qed_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey},
};
use qed_data::{
    guta::end_cap_input::SubmitUserEndCapNonProofInput,
    qblock::cmds::deploy_contract::QBCDeployContract,
};
use qed_prover::ups::{
    circuit_manager::core::QEDUPSStepCircuitManager, session::UserProvingSessionManager,
};
use qed_store::{
    config::store_config::QEDHasher,
    controllers::local::{
        proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore,
    },
    store::imm::cmd_processor::QEDReadCommandProcessorSync,
    traits::qdatastore::qmetadata::QMetaDataStoreReaderSync,
};
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use serde::{Deserialize, Serialize};

use crate::rpc::{
    provider::{QUserRpcProvider, RpcConfig, RpcProvider},
    request::{QDeployContractRPCRequest, QRegisterUserRPCRequest, QSubmitEndCapRPCRequest},
};

use super::{
    args::{ContractCallArgs, WalletSessionArgs},
    deploy_contract::gen_contract_deploy_and_circuits_for_functions,
    utils::prove_func,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub struct UserSessionStateManager {
    pub mgr: UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>,
    pub user_id: u64,
    pub nonce: F,
    pub current_checkpoint_id: u64,
}

impl UserSessionStateManager {
    pub fn new(
        user_id: u64,
        nonce: F,
        checkpoint_id: u64,
        st_provider: RpcProvider,
        circuit_info: SessionCircuitInfoStore<F>,
        main_circuits: &QEDUPSStepCircuitManager<C, D>,
    ) -> anyhow::Result<UserSessionStateManager> {
        tracing::info!("create local proving session store");
        let lps = QEDLocalProvingSessionStore::new_at(
            st_provider.clone(),
            F::from_noncanonical_u64(checkpoint_id),
            F::from_canonical_u64(user_id),
            nonce,
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
        );

        tracing::info!("create ups manager");
        let mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
            lps,
            circuit_info,
            main_circuits.ups_circuit_whitelist_root,
        )?;

        Ok(UserSessionStateManager {
            mgr,
            user_id,
            nonce: nonce,
            current_checkpoint_id: checkpoint_id,
        })
    }

    pub fn new_with_dummy_mgr(
        st_provider: RpcProvider,
        circuit_info: SessionCircuitInfoStore<F>,
    ) -> anyhow::Result<UserSessionStateManager> {
        tracing::info!("create dummy local proving session store");
        let lps = QEDLocalProvingSessionStore::new_at(
            st_provider.clone(),
            F::from_noncanonical_u64(0),
            F::from_canonical_u64(0),
            F::from_canonical_u64(0),
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
        );

        tracing::info!("create ups manager");
        let mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new_dummy(lps, circuit_info)?;

        Ok(UserSessionStateManager {
            mgr,
            user_id: 0,
            nonce: F::from_canonical_u64(0),
            current_checkpoint_id: 0,
        })
    }
}

pub struct WalletSession {
    pub wallet: SimpleQEDZKSignatureManager<C, D>,
    wallet_keys_store: hashbrown::HashMap<QHashOut<F>, ZKPublicKeyInfo<F>>,
    pub main_circuits: QEDUPSStepCircuitManager<C, D>,
    pub circuit_info: SessionCircuitInfoStore<F>,
    pub st_provider: RpcProvider,

    pub current_user_pk_hash: QHashOut<F>,
    pub user_session_mgr: UserSessionStateManager,
}

impl WalletSession {
    pub fn new(rpc_config: &RpcConfig) -> anyhow::Result<Self> {
        tracing::info!("init rpc provider");
        let st_provider = RpcProvider::new_with_config(rpc_config)?;

        tracing::info!("init wallet");
        let wallet = SimpleQEDZKSignatureManager::<C, D>::new();

        tracing::info!("init ups step circuit manager");
        let main_circuits =
            QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST);
        let mut circuit_info = SessionCircuitInfoStore::new();

        tracing::info!("register ZKSignature circuit info");
        circuit_info.register_circuit(
            LocalCircuitType::SimpleZKSignature.into(),
            wallet.circuit.get_fingerprint(),
            wallet.circuit.get_verifier_config_ref().into(),
        );

        main_circuits.register_info(&mut circuit_info);

        let user_session_mgr =
            UserSessionStateManager::new_with_dummy_mgr(st_provider.clone(), circuit_info.clone())?;

        Ok(WalletSession {
            wallet,
            wallet_keys_store: hashbrown::HashMap::new(),
            main_circuits,
            circuit_info,
            st_provider,
            current_user_pk_hash: QHashOut::<F>::default(),
            user_session_mgr,
        })
    }

    pub fn register_user(&mut self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        let pk_info = self
            .wallet
            .add_private_key_get_info(SimpleQEDPrivateKey { private_key });
        let pk_hash = pk_info.qfhash::<QEDHasher>();
        self.st_provider.register_user(QRegisterUserRPCRequest {
            public_key: pk_info,
        })?;
        self.wallet_keys_store.insert(pk_hash, pk_info);

        tracing::info!("user `{}` registered", pk_hash);
        tracing::warn!("please use this user after 2 checkpoints!");
        Ok(pk_hash)
    }

    pub fn add_user(&mut self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        let pk_info = self
            .wallet
            .add_private_key_get_info(SimpleQEDPrivateKey { private_key });
        let pk_hash = pk_info.qfhash::<QEDHasher>();
        let user_id = self
            .st_provider
            .get_user_id(pk_info.public_key_param)
            .map_err(|e| {
                anyhow::format_err!(
                    "Error `{}`. user {} not found, please register it first",
                    e.to_string(),
                    pk_info.qfhash::<QEDHasher>().to_string()
                )
            })?;
        self.wallet_keys_store.insert(pk_hash, pk_info);

        tracing::info!(
            "user {} {} added",
            user_id,
            pk_info.qfhash::<QEDHasher>().to_string()
        );

        Ok(pk_hash)
    }

    pub fn switch_user(&mut self, pk_hash: QHashOut<F>) -> anyhow::Result<()> {
        tracing::info!("switch user {}", pk_hash.to_string());
        let pk_info = self
            .wallet_keys_store
            .get(&pk_hash)
            .ok_or_else(|| anyhow::format_err!("switch user {} not found", pk_hash.to_string()))?;
        let user_id = self.st_provider.get_user_id(pk_info.public_key_param)?;
        if !self.wallet.contains_key(pk_info.qfhash::<QEDHasher>()) {
            tracing::error!(
                "user {} not found, please add it first!",
                pk_info.qfhash::<QEDHasher>().to_string()
            );
            Err(anyhow::format_err!(
                "user {} not found, please add it first!",
                pk_info.qfhash::<QEDHasher>().to_string()
            ))
        } else {
            tracing::info!(
                "switched user {} to user {} {}",
                self.user_session_mgr.user_id,
                user_id,
                pk_info.qfhash::<QEDHasher>().to_string()
            );
            self.st_provider.current_user_id = user_id;
            self.current_user_pk_hash = pk_hash;
            Ok(())
        }
    }

    pub fn start_session(&mut self) -> anyhow::Result<()> {
        tracing::info!("start new user proving session");
        let latest_l2_block_state = self.st_provider.resolve_get_latest_l2_block_state()?;
        let latest_nonce = self
            .st_provider
            .get_user_leaf_data(
                latest_l2_block_state.checkpoint_id,
                self.st_provider.current_user_id,
            )?
            .nonce
            + F::from_noncanonical_u64(1);

        if self.user_session_mgr.user_id == self.st_provider.current_user_id
            && latest_nonce == self.user_session_mgr.nonce
            && latest_l2_block_state.checkpoint_id == self.user_session_mgr.current_checkpoint_id
        {
            tracing::info!("user session manager already exists");
        } else {
            tracing::info!("create new user session manager");
            self.user_session_mgr = UserSessionStateManager::new(
                self.st_provider.current_user_id,
                latest_nonce + F::from_noncanonical_u64(1),
                latest_l2_block_state.checkpoint_id,
                self.st_provider.clone(),
                self.circuit_info.clone(),
                &self.main_circuits,
            )?;
        };

        tracing::info!("local proving ups start");

        self.user_session_mgr
            .mgr
            .prove_ups_start(&self.main_circuits)?;

        Ok(())
    }

    pub fn prove_contract_call(
        &mut self,
        contract_call_arg: ContractCallArgs,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "prove contract call at contract {}, method {}",
            contract_call_arg.contract_id,
            contract_call_arg.method_name
        );
        prove_func(
            &self.st_provider,
            &self.main_circuits,
            &mut self.user_session_mgr.mgr,
            contract_call_arg.contract_id,
            &contract_call_arg.method_name,
            contract_call_arg
                .inputs
                .iter()
                .map(|x| F::from_noncanonical_u64(*x))
                .collect(),
        )
    }

    pub fn prove_contract_calls(
        &mut self,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> anyhow::Result<()> {
        for contract_call_arg in contract_call_args {
            self.prove_contract_call(contract_call_arg)?;
        }
        Ok(())
    }

    pub fn get_sig_hash(&self, network_magic: u64) -> anyhow::Result<QHashOut<F>> {
        let sighash = self
            .user_session_mgr
            .mgr
            .get_sighash(network_magic, self.user_session_mgr.nonce);
        tracing::info!("get sig hash: {}", sighash.to_string());
        Ok(sighash)
    }

    pub fn get_zk_signature(
        &self,
        sighash: QHashOut<F>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        tracing::info!("get zk signature proof");
        let pk_info = self
            .wallet_keys_store
            .get(&self.current_user_pk_hash)
            .ok_or(anyhow::format_err!(
                "user {} not found, cannot sign",
                self.current_user_pk_hash.to_string()
            ))?;
        let signature_proof = self
            .wallet
            .zk_sign_for_public_key(pk_info.qfhash::<QEDHasher>(), sighash)?;
        Ok(signature_proof)
    }

    pub fn get_end_cap_proof(
        &mut self,
        signature_proof: ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        tracing::info!("get end cap proof");

        self.user_session_mgr
            .mgr
            .proof_tree_state
            .finalize_tree(&self.main_circuits.proof_tree_agg_circuits)?;

        let public_key_param = self
            .wallet_keys_store
            .get(&self.current_user_pk_hash)
            .ok_or(anyhow::format_err!(
                "user {} not found, cannot get public key param",
                self.user_session_mgr.user_id
            ))?
            .public_key_param;

        tracing::info!(
        "prove end cap with network magic {:x}, nonce {}, fingerprint {}, public key param {}, signature proof {:?}",
            QED_NETWORK_MAGIC_REGTEST,
            self.user_session_mgr.nonce,
            self.wallet.circuit.get_fingerprint(),
            public_key_param,
            signature_proof.public_inputs
        );
        let end_cap_proof = self.user_session_mgr.mgr.prove_end_cap(
            &self.main_circuits,
            QED_NETWORK_MAGIC_REGTEST,
            self.user_session_mgr.nonce,
            self.wallet.circuit.get_fingerprint(),
            public_key_param,
            signature_proof,
            self.wallet.circuit.get_verifier_config_ref().to_owned(),
        )?;

        Ok(end_cap_proof)
    }

    pub fn get_user_ec_input(&mut self) -> anyhow::Result<SubmitUserEndCapNonProofInput<F>> {
        self.user_session_mgr.mgr.get_api_input()
    }

    pub fn sign_and_submit(&mut self) -> anyhow::Result<()> {
        let sighash = self.get_sig_hash(QED_NETWORK_MAGIC_REGTEST)?;
        tracing::info!("zk sign for signhash: {}", sighash.to_string());

        let signature_proof = self.get_zk_signature(sighash)?;

        let end_cap_proof = self.get_end_cap_proof(signature_proof)?;

        let user_ec_input = self.get_user_ec_input()?;
        tracing::info!(
            "get user ec input: {}",
            serde_json::to_string_pretty(&user_ec_input)?
        );
        let req = QSubmitEndCapRPCRequest {
            user_ec_input,
            proof: end_cap_proof,
        };

        self.st_provider.submit_end_cap_proof::<F>(req)?;

        Ok(())
    }

    pub fn get_deploy_contract_cmd(
        &self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> anyhow::Result<QBCDeployContract<F>> {
        let deployer = self.current_user_pk_hash;
        if !self.wallet_keys_store.contains_key(&deployer) {
            tracing::error!(
                "user {} not found in wallet, please register or add it first",
                deployer.to_string()
            );
            return Err(anyhow::format_err!(
                "user {} not found in wallet, please register or add it first",
                deployer.to_string()
            ));
        }

        let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

        let (_result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
            deployer,
            contract_state_tree_height as u8,
            &circuit_defs,
        )?;
        Ok(deploy_cmd)
    }

    pub fn deploy_contract(
        &mut self,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> anyhow::Result<()> {
        let deploy_cmd = self.get_deploy_contract_cmd(circuit_defs)?;

        self.st_provider
            .deploy_contract::<F>(QDeployContractRPCRequest {
                deploy_contract: deploy_cmd,
            })?;

        Ok(())
    }

    pub fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let pk_info = self
            .wallet
            .get_public_key_info(SimpleQEDPrivateKey { private_key });
        Ok(pk_info)
    }

    pub fn get_random_keypair(&self) -> anyhow::Result<WalletKeyPair> {
        let private_key = QHashOut::<F>::rand();
        let pk_info = self.get_zk_public_key(private_key)?;
        Ok(WalletKeyPair {
            private_key,
            public_key: pk_info,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletKeyPair {
    pub private_key: QHashOut<F>,
    pub public_key: ZKPublicKeyInfo<F>,
}

pub fn run(args: WalletSessionArgs) -> anyhow::Result<()> {
    let rpc_config: RpcConfig = serde_json::from_str(&std::fs::read_to_string(args.rpc_config)?)?;
    let private_key = QHashOut::<F>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let contract_call_args: Vec<ContractCallArgs> =
        serde_json::from_str(&std::fs::read_to_string(args.contract_calls)?)?;

    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let pk_hash = wallet_session.add_user(private_key)?;
    wallet_session.switch_user(pk_hash)?;
    wallet_session.start_session()?;
    wallet_session.prove_contract_calls(contract_call_args)?;
    wallet_session.sign_and_submit()?;

    Ok(())
}

mod tests {
    use super::*;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_scenario0() -> anyhow::Result<()> {
        qed_rollup_utils::setup_logging("info".to_string())?;
        tracing::info!("test_scenario0");
        let project_path = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| anyhow::format_err!("Error `{}`, cannot get CARGO_MANIFEST_DIR env", e))?;

        let private_key0 = QHashOut::<GoldilocksField>::from_str(
            "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",
        )?;
        let private_key8388608 = QHashOut::<GoldilocksField>::from_str(
            "f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d",
        )?;

        let rpc_config: RpcConfig = serde_json::from_str(&std::fs::read_to_string(
            Path::new(&project_path).join("../rpc.config"),
        )?)?;

        let circuit_defs =
            serde_json::from_str::<Vec<DPNFunctionCircuitDefinition>>(&std::fs::read_to_string(
                Path::new(&project_path).join("../examples/target/examples.json"),
            )?)?;

        let mut wallet_session = super::WalletSession::new(&rpc_config)?;

        let user0 = wallet_session.register_user(private_key0)?;
        let user8388608 = wallet_session.register_user(private_key8388608)?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        wallet_session.switch_user(user0)?;

        wallet_session.deploy_contract(circuit_defs)?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        // user0 mint 1000
        wallet_session.start_session()?;
        wallet_session.prove_contract_call(ContractCallArgs {
            contract_id: 0,
            method_name: "simple_mint".to_string(),
            inputs: vec![1000],
        })?;
        wallet_session.sign_and_submit()?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));
        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        // user0 transfer 500 to user8388608
        wallet_session.start_session()?;
        wallet_session.prove_contract_call(ContractCallArgs {
            contract_id: 0,
            method_name: "simple_transfer".to_string(),
            inputs: vec![8388608, 500],
        })?;
        wallet_session.sign_and_submit()?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));
        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        // user8388608 claim
        wallet_session.switch_user(user8388608)?;
        wallet_session.start_session()?;
        wallet_session.prove_contract_call(ContractCallArgs {
            contract_id: 0,
            method_name: "simple_claim".to_string(),
            inputs: vec![0],
        })?;
        wallet_session.sign_and_submit()?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));
        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        // user8388608 transfer 500 to user0
        wallet_session.start_session()?;
        wallet_session.prove_contract_call(ContractCallArgs {
            contract_id: 0,
            method_name: "simple_transfer".to_string(),
            inputs: vec![0, 500],
        })?;
        wallet_session.sign_and_submit()?;

        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));
        wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(5));

        Ok(())
    }
}
