use std::{collections::HashMap, str::FromStr};

use crate::{
    dpn::circuits::cfc::DapenContractFunctionCircuit, local::{
        args::{ContractCallArgs, JobLocation, SignType},
        provider::{ProveProxyRpcProvider, ProveProxyRpcTrait},
    }, session::{build_claim_calls_for_multi_checkpoints, ProofWithCheckpoint, MINING_REWARDS_CONTRACT_ID}, ups::{
        circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager},
        session::UserProvingSessionManager,
    }, wallet::{
        memory_wallet::QEDMemoryWallet,
        simple_sign::StateReader,
        software_defined_circuit::{
            PSoftwareDefinedSignatureWitnessInput, QSoftwareDefinedSignatureGadget,
            QSoftwareDefinedSignatureWitnessInput, SoftwareDefinedSignature,
            SoftwareDefinedSignatureGadget, SoftwareDefinedSignatureWitnessInput,
        },
    }
};
use dashmap::DashMap;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
};
use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_core::{
    config::network_constants::{
        MAX_CONTRACT_STATE_TREE_HEIGHT, QED_NETWORK_MAGIC_REGTEST, TOKEN_CONTRACT_ID, UPS_SESSION_PROOF_TREE_HEIGHT
    },
    data::qhashout::QHashOut,
    traits::to_qfelts::ToQFelts,
    ups::circuits::LocalCircuitType,
};
use qed_crypto::{hash::traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable}, signature::zk::data::ZKPublicKeyInfo};
use qed_data::{
    config::store_config::QEDHasher,
    qdata::{user_contract_state::UserContractState, checkpoint::QEDL2BlockState},
    qstore::imm::{
        cmd::{
            QSRCmdGetContractCodeDefinition, QSRCmdGetUserLeafData, QSRHashCmd,
            QSRHashCmdGetUserContractStateTreeRoot,
        },
        cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut},
    },
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync,
    },
};
use qed_data::{
    qblock::cmds::deploy_contract::QBCDeployContract, qdata::contract::ContractCodeDefinition,
};
use qed_store::controllers::local::{
    proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore,
};
use qedlang_core::dpn::{
    contract::{cfc_code_definition_to_dapen_fc, dapen_fc_to_cfc_code_definition}, vm::def::DPNFunctionCircuitDefinition,
};
use qed_core::job::id::{ProvingJobCircuitType, VariableHeightRewardMerkleProof};
use serde::{Deserialize, Serialize};

use crate::local::{
    args::WalletSessionArgs,
    provider::{QUserRpcProvider, RpcConfig, RpcProvider},
        request::{QDeployContractRPCRequest, QRegisterUserRPCRequest, QSubmitEndCapRPCRequest},
};
use crate::session::get_job_proof;
use crate::local::args::JobInfo;

pub fn gen_contract_deploy_and_circuits_for_functions<C: GenericConfig<D>, const D: usize>(
    deployer: QHashOut<C::F>,
    contract_state_tree_height: u8,
    defs: &[DPNFunctionCircuitDefinition],
) -> anyhow::Result<(
    Vec<DapenContractFunctionCircuit<C, D>>,
    QBCDeployContract<C::F>,
)>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    let code_defs = defs
        .iter()
        .map(|x| dapen_fc_to_cfc_code_definition(x))
        .collect::<Vec<_>>();
    let mut fingerprints = Vec::with_capacity(defs.len() * 2);
    let circuits = defs
        .iter()
        .map(|x| {
            let c = DapenContractFunctionCircuit::<C, D>::new(
                x,
                contract_state_tree_height as usize,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                false,
            );
            fingerprints.push(c.get_fingerprint());

            let inputs_outputs_combo =
                ((x.circuit_outputs.len() as u64) << 32u64) | (x.circuit_inputs.len() as u64);
            fingerprints.push(QHashOut::from_values(
                x.method_id as u64,
                inputs_outputs_combo,
                0,
                0,
            ));
            c
        })
        .collect::<Vec<_>>();

    let deploy = QBCDeployContract {
        deployer,
        code_definition: ContractCodeDefinition {
            state_tree_height: contract_state_tree_height as u16,
            functions: code_defs,
        },
        function_whitelist: fingerprints,
    };

    Ok((circuits, deploy))
}

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub async fn prove_func<R: QEDReadCommandProcessorSync<F> + Send + Sync>(
    st: &R,
    circuit_mgr: &QCircuitManager<C, D>,
    mgr: &mut UserProvingSessionManager<F, PoseidonHash, R, C, D>,
    contract_id: u64,
    fn_name: &str,
    inputs: Vec<F>,
) -> anyhow::Result<()> {
    let contract_code = st
        .resolve_get_contract_code(&QSRCmdGetContractCodeDefinition { contract_id })
        .await?;

    circuit_mgr
        .register_contract_circuits(contract_id, &contract_code)
        .await?;

    let method_id = circuit_mgr
        .get_method_id(contract_id, fn_name.to_string())
        .await?;

    let dapen_fc = cfc_code_definition_to_dapen_fc(&contract_code.functions[method_id as usize])?;

    mgr.prove_standard_call(
        circuit_mgr,
        F::from_canonical_u64(contract_id),
        method_id as u32,
        &dapen_fc,
        inputs,
    )
    .await
}

pub enum UserState {
    Active,
    Registering,
    InActive,
}

pub struct UserSessionStateManager {
    pub user_state: UserState,
    pub rpc_provider: RpcProvider,
    pub mgr: UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>,
    pub user_id: u64,
    pub nonce: F,
    pub current_checkpoint_id: u64,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl UserSessionStateManager {
    pub async fn new(
        user_id: u64,
        nonce: F,
        checkpoint_id: u64,
        st_provider: RpcProvider,
        circuit_info: SessionCircuitInfoStore<F>,
        main_circuits: &QCircuitManager<C, D>,
    ) -> anyhow::Result<UserSessionStateManager> {
        tracing::info!("create local proving session store");
        let mut rpc_provider = st_provider.clone();
        rpc_provider.current_user_id = user_id;
        let lps = QEDLocalProvingSessionStore::new_at(
            rpc_provider.clone(),
            F::from_noncanonical_u64(checkpoint_id),
            F::from_canonical_u64(user_id),
            nonce,
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
        );

        tracing::info!("create ups manager");
        let mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
            lps,
            circuit_info,
            main_circuits.ups_circuit_whitelist_root().await?,
        )
        .await?;

        Ok(UserSessionStateManager {
            user_state: UserState::Active,
            rpc_provider,
            mgr,
            user_id,
            nonce: nonce,
            current_checkpoint_id: checkpoint_id,
        })
    }

    pub async fn new_with_dummy_mgr(
        user_state: UserState,
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
        let mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new_dummy(lps, circuit_info)
            .await?;

        Ok(UserSessionStateManager {
            user_state,
            rpc_provider: st_provider.clone(),
            mgr,
            user_id: 0,
            nonce: F::from_canonical_u64(0),
            current_checkpoint_id: 0,
        })
    }
}

pub struct WalletSession {
    pub wallet: QEDMemoryWallet,
    wallet_keys_store: DashMap<QHashOut<F>, ZKPublicKeyInfo<F>>,
    // pub main_circuits: QCircuitManager<C, D>,
    pub circuit_info: SessionCircuitInfoStore<F>,
    pub st_provider: RpcProvider,

    pub user_session_mgrs: DashMap<QHashOut<F>, UserSessionStateManager>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl WalletSession {
    pub async fn new(rpc_config: &RpcConfig) -> anyhow::Result<Self> {
        tracing::info!("init rpc provider");
        let st_provider = RpcProvider::new_with_config(rpc_config)?;

        tracing::info!("init wallet");
        tracing::info!("init ups step circuit manager");
        let mut main_circuits = Vec::new();

        for proxy_url in rpc_config.prove_proxy_url.iter() {
            if let Ok(main_circuit) = ProveProxyRpcProvider::new_with_config(proxy_url.to_string()).await {
                main_circuits.push(QCircuitManager::Rpc(main_circuit));
            } else {
                tracing::info!("prove proxy url `{}` is invalid, skip", proxy_url);
            }
        }
        if main_circuits.is_empty() {
            tracing::warn!("no valid prove proxy url, use local circuit manager");
            main_circuits.push(QCircuitManager::Local(QEDUPSStepCircuitManager::<C, D>::new_with_config(
                QED_NETWORK_MAGIC_REGTEST,
            )));
        }

        let mut circuit_info = SessionCircuitInfoStore::new();

        tracing::info!("register ZKSignature circuit info");
        circuit_info.register_circuit(
            LocalCircuitType::SimpleZKSignature.into(),
            main_circuits[0].zk_circuit_fingerprint().await?,
            main_circuits[0].zk_circuit_verifier_config().await?.into(),
        );

        circuit_info.register_circuit(
            LocalCircuitType::SimpleSecp256K1.into(),
            main_circuits[0].secp_circuit_fingerprint().await?,
            main_circuits[0].secp_circuit_verifier_config().await?.into(),
        );

        for main_circuit in main_circuits.iter() {
            main_circuit.register_info(&mut circuit_info).await;
        }

        let wallet = QEDMemoryWallet::new(main_circuits);

        Ok(WalletSession {
            wallet,
            wallet_keys_store: DashMap::new(),
            // main_circuits,
            circuit_info,
            st_provider,
            user_session_mgrs: DashMap::new(),
        })
    }

    pub async fn register_user(&mut self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        self.register_user_with_type(private_key, SignType::SECP256K1Sign, None)
            .await
    }

    pub async fn register_user_with_type(
        &mut self,
        private_key: QHashOut<F>,
        sign_type: SignType,
        fingerprint: Option<QHashOut<F>>,
    ) -> anyhow::Result<QHashOut<F>> {
        let pk_info = match sign_type {
            SignType::ZKSign => self.wallet.add_zk_private_key(private_key).await?,
            SignType::SECP256K1Sign => self.wallet.add_secp_private_key(private_key).await?,
            SignType::SoftwareDefinedSign => {
                self.wallet
                    .add_software_defined_private_key(
                        private_key,
                        fingerprint.ok_or(anyhow::format_err!(
                            "software defined sign need fingerprint"
                        ))?,
                    )
                    .await?
            }
        };
        let public_key = pk_info.qfhash::<QEDHasher>();

        if let Ok(user_id) = self.st_provider.get_user_id(public_key).await {
            tracing::info!("user `{}` already registered with id {}", public_key, user_id);
            return Ok(public_key);
        }

        self.st_provider
            .register_user(QRegisterUserRPCRequest {
                public_key: pk_info,
            })
            .await?;

        tracing::info!("user `{}` registered", public_key);
        tracing::warn!("please add this user after 2 checkpoints!");
        Ok(public_key)
    }

    pub async fn add_user(&mut self, private_key: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        self.add_user_with_type(private_key, SignType::SECP256K1Sign, None)
            .await
    }

    pub async fn add_user_with_type(
        &mut self,
        private_key: QHashOut<F>,
        sign_type: SignType,
        fingerprint: Option<QHashOut<F>>,
    ) -> anyhow::Result<QHashOut<F>> {
        let pk_info = match sign_type {
            SignType::ZKSign => self.wallet.add_zk_private_key(private_key).await?,
            SignType::SECP256K1Sign => self.wallet.add_secp_private_key(private_key).await?,
            SignType::SoftwareDefinedSign => {
                self.wallet
                    .add_software_defined_private_key(
                        private_key,
                        fingerprint.ok_or(anyhow::format_err!(
                            "software defined sign need fingerprint"
                        ))?,
                    )
                    .await?
            }
        };
        let public_key = pk_info.qfhash::<QEDHasher>();
        let checkpoint_id = self
            .st_provider
            .get_latest_l2_block_state()
            .await?
            .checkpoint_id;
        tracing::info!(
            "add user {} with type {:?}, on checkpoint_id {}",
            public_key.to_string(),
            sign_type,
            checkpoint_id
        );

        let (user_id, nonce, user_state) = match self.st_provider.get_user_id(public_key).await {
            Ok(user_id) => match self
                .st_provider
                .get_user_leaf_data(checkpoint_id, user_id)
                .await
            {
                Ok(user_leaf_data) => (user_id, user_leaf_data.nonce, UserState::Active),
                Err(e) => {
                    tracing::warn!(
                        "can not get user id for user `{}`, wait for 2 blocks after register: {}",
                        public_key.to_string(),
                        e
                    );
                    (user_id, F::ZERO, UserState::Registering)
                }
            },
            Err(e) => {
                tracing::warn!(
                    "can not get user id for user `{}`, please register it first: {}",
                    public_key.to_string(),
                    e
                );
                (0, F::ZERO, UserState::InActive)
            }
        };

        if !self.wallet_keys_store.contains_key(&public_key) {
            self.wallet_keys_store.insert(public_key, pk_info);

            match user_state {
                UserState::Active => {
                    self.user_session_mgrs.insert(
                        public_key,
                        UserSessionStateManager::new(
                            user_id,
                            nonce + F::from_canonical_u64(1),
                            checkpoint_id,
                            self.st_provider.clone(),
                            self.circuit_info.clone(),
                            &self.wallet.random_circuit_manager(),
                        )
                        .await?,
                    );
                }
                UserState::Registering | UserState::InActive => {
                    self.user_session_mgrs.insert(
                        public_key,
                        UserSessionStateManager::new_with_dummy_mgr(
                            user_state,
                            self.st_provider.clone(),
                            self.circuit_info.clone(),
                        )
                        .await?,
                    );
                }
            }

            tracing::info!("user {} added", pk_info.qfhash::<QEDHasher>().to_string());
        } else {
            tracing::info!(
                "user {} already added",
                pk_info.qfhash::<QEDHasher>().to_string()
            );
        }

        Ok(public_key)
    }

    pub async fn exec_contract_call(
        &self,
        public_key: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> anyhow::Result<()> {
        self.exec_contract_call_with_sign_type(
            public_key,
            contract_call_args,
            SignType::SECP256K1Sign,
            None,
            None,
            vec![],
        )
            .await
    }

    pub async fn exec_contract_call_with_sign_type(
        &self,
        public_key: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
        sign_type: SignType,
        fingerprint: Option<QHashOut<F>>,
        sig_contract_id: Option<u64>,
        sign_inputs: Vec<u64>,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "exec contract call: {}",
            serde_json::to_string_pretty(&contract_call_args)?
        );
        let result = self.st_provider.get_latest_l2_block_state().await?;
        tracing::info!("start session on checkpoint: {}", result.checkpoint_id);
        self.start_session(public_key).await?;
        tracing::info!("prove contract calls");
        self.prove_contract_calls(public_key, contract_call_args)
            .await?;
        let result = self.st_provider.get_latest_l2_block_state().await?;
        tracing::info!("sign and submit on checkpoint: {}", result.checkpoint_id);
        self.sign_and_submit_with_sign_type(
            public_key,
            sign_type,
            fingerprint,
            sig_contract_id,
            sign_inputs,
        )
            .await?;
        Ok(())
    }


    pub async fn start_session(&self, public_key: QHashOut<F>) -> anyhow::Result<()> {
        tracing::info!("start new user proving session");
        tracing::info!("get user session manager: {:?}", self.user_session_mgrs.iter().map(|item| item.key().to_string()).collect::<Vec<_>>());
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;
        
        let latest_l2_block_state = user_session_mgr.rpc_provider.get_realm_latest_l2_block_state().await?;
        let global_latest_l2_block_state = self.st_provider.get_latest_l2_block_state().await?;

        if latest_l2_block_state.checkpoint_id <= global_latest_l2_block_state.checkpoint_id {
            tracing::info!(
                "block state: user_id {}, realm latest checkpoint {}, coordinator checkpoint {}",
                user_session_mgr.user_id,
                latest_l2_block_state.checkpoint_id,
                global_latest_l2_block_state.checkpoint_id
            );
        } else {
            tracing::error!(
                "realm latest checkpoint {} is ahead coordinator checkpoint {}",
                latest_l2_block_state.checkpoint_id,
                global_latest_l2_block_state.checkpoint_id
            );
            return Err(anyhow::format_err!(
                "realm latest checkpoint {} is ahead of coordinator checkpoint {}",
                latest_l2_block_state.checkpoint_id,
                global_latest_l2_block_state.checkpoint_id
            ))
        };

        match user_session_mgr.user_state {
            UserState::Active => {
                let latest_nonce = self
                    .st_provider
                    .get_user_leaf_data(
                        latest_l2_block_state.checkpoint_id,
                        user_session_mgr.user_id,
                    )
                    .await?
                    .nonce
                    + F::from_noncanonical_u64(1);

                if latest_nonce == user_session_mgr.nonce
                    && latest_l2_block_state.checkpoint_id == user_session_mgr.current_checkpoint_id
                    && user_session_mgr
                        .mgr
                        .current_ups_header
                        .current_state
                        .tx_count
                        == F::ZERO
                {
                    tracing::info!("user session manager already exists");
                } else {
                    tracing::info!("create new user session manager");
                    *user_session_mgr = UserSessionStateManager::new(
                        user_session_mgr.user_id,
                        user_session_mgr.nonce,
                        latest_l2_block_state.checkpoint_id,
                        self.st_provider.clone(),
                        self.circuit_info.clone(),
                        &self.wallet.random_circuit_manager(),
                    )
                    .await?;
                };
            }
            UserState::InActive | UserState::Registering => {
                let user_id = self.st_provider.get_user_id(public_key).await.map_err(|e| {
                    anyhow::format_err!(
                        "can not get user id for user `{}`, please add it first: {}",
                        public_key.to_string(),
                        e
                    )
                })?;
                let checkpoint_id = latest_l2_block_state.checkpoint_id;
                let user_leaf_data = self
                    .st_provider
                    .get_user_leaf_data(checkpoint_id, user_id).await
                    .map_err(|e| {
                    anyhow::format_err!(
                        "can not get user id for user `{}`, please wait for 2 blocks after register: {}",
                        public_key.to_string(),
                        e
                    )
                })?;
                *user_session_mgr = UserSessionStateManager::new(
                    user_id,
                    user_leaf_data.nonce + F::from_canonical_u64(1),
                    checkpoint_id,
                    self.st_provider.clone(),
                    self.circuit_info.clone(),
                    &self.wallet.random_circuit_manager(),
                )
                .await?;
            }
        }

        tracing::info!("local proving ups start");

        tracing::info!("user session manager nonce: {}", user_session_mgr.nonce);

        user_session_mgr
            .mgr
            .prove_ups_start(&self.wallet.random_circuit_manager())
            .await?;

        Ok(())
    }

    pub async fn prove_contract_call(
        &self,
        public_key: QHashOut<F>,
        contract_call_arg: ContractCallArgs,
    ) -> anyhow::Result<()> {
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;
        tracing::info!(
            "prove contract call at contract {}, method {}",
            contract_call_arg.contract_id,
            contract_call_arg.method_name
        );
        prove_func(
            &user_session_mgr.rpc_provider.clone(),
            &self.wallet.random_circuit_manager(),
            &mut user_session_mgr.mgr,
            contract_call_arg.contract_id,
            &contract_call_arg.method_name,
            contract_call_arg
                .inputs
                .iter()
                .map(|x| F::from_noncanonical_u64(*x))
                .collect(),
        )
        .await?;
        user_session_mgr.mgr.prove_burn_fee(&self.wallet.random_circuit_manager()).await?;
        Ok(())
    }

    pub async fn prove_contract_calls(
        &self,
        public_key: QHashOut<F>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> anyhow::Result<()> {
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;
        for contract_call_arg in contract_call_args {
            tracing::info!(
                "prove contract call at contract {}, method {}",
                contract_call_arg.contract_id,
                contract_call_arg.method_name
            );
            prove_func(
                &user_session_mgr.rpc_provider.clone(),
                &self.wallet.random_circuit_manager(),
                &mut user_session_mgr.mgr,
                contract_call_arg.contract_id,
                &contract_call_arg.method_name,
                contract_call_arg
                    .inputs
                    .iter()
                    .map(|x| F::from_noncanonical_u64(*x))
                    .collect(),
            )
            .await?;
        }
        user_session_mgr.mgr.prove_burn_fee(&self.wallet.random_circuit_manager()).await?;
        Ok(())
    }

    pub async fn sign_and_submit(&self, public_key: QHashOut<F>) -> anyhow::Result<()> {
        self.sign_and_submit_with_sign_type(public_key, SignType::SECP256K1Sign, None, None, vec![])
            .await
    }

     pub async fn sign_and_submit_with_sign_type(
        &self,
        public_key: QHashOut<F>,
        sign_type: SignType,
        fingerprint: Option<QHashOut<F>>,
        sig_contract_id: Option<u64>,
        sign_inputs: Vec<u64>,
    ) -> anyhow::Result<()> {
        tracing::info!("🔔sign and submit with sign type: {:?}", sign_type);

        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;

        let sighash = user_session_mgr
            .mgr
            .get_sighash(QED_NETWORK_MAGIC_REGTEST, user_session_mgr.nonce);

        let pk_info = self
            .wallet_keys_store
            .get(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;
        tracing::info!("zk sign for signhash: {}", sighash.to_string());
        let signature_proof = match sign_type {
            SignType::ZKSign => {
                self.wallet
                    .zk_sign_for_public_key(pk_info.public_key_param, sighash)
                    .await?
            }
            SignType::SECP256K1Sign => {
                self.wallet
                    .zk_sign_secp256k1(pk_info.public_key_param, sighash)
                    .await?
            }
            SignType::SoftwareDefinedSign => {
                if let Some(fingerprint) = fingerprint {
                    let mut sdc = self
                        .wallet
                        .software_defined_circuits
                        .get_mut(&fingerprint)
                        .ok_or(anyhow::format_err!(
                            "software defined circuit `{}` not found",
                            fingerprint.to_string()
                        ))?;

                    let cfc_call_inputs = sign_inputs
                        .iter()
                        .map(|x| F::from_noncanonical_u64(*x))
                        .collect::<Vec<_>>();

                    let input = match &sdc.signature_gadget {
                        SoftwareDefinedSignatureGadget::QED(q_gadget) => {
                            let cfc_proof_input = user_session_mgr
                                .mgr
                                .exec_contract_call(
                                    F::from_noncanonical_u64(sig_contract_id.unwrap_or_default()),
                                    &q_gadget.input.fn_def,
                                    cfc_call_inputs,
                                )
                                .await?;
                            SoftwareDefinedSignatureWitnessInput::QED(
                                QSoftwareDefinedSignatureWitnessInput {
                                    cfc_input: cfc_proof_input,
                                },
                            )
                        }
                        SoftwareDefinedSignatureGadget::PLONKY2(_p_gadget) => {
                            let user_id = user_session_mgr.user_id;
                            let checkpoint_id = user_session_mgr.current_checkpoint_id;
                            let user_leaf = user_session_mgr
                                .mgr
                                .lps
                                .cmd_store
                                .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData {
                                    checkpoint_id: checkpoint_id,
                                    user_id,
                                })
                                .await?;
                            let checkpoint_tree_root = self
                                .st_provider
                                .get_checkpoint_tree_root(checkpoint_id)
                                .await?;

                            let transaction_record =
                                user_session_mgr.mgr.lps.transaction_records.last().ok_or(
                                    anyhow::format_err!(
                                        "you must exec at least one contract call before sign"
                                    ),
                                )?;
                            tracing::info!(
                                "transaction_record: {}",
                                serde_json::to_string_pretty(&transaction_record)?
                            );

                            let user_contract_state = UserContractState::new(
                                checkpoint_tree_root,
                                user_leaf,
                                transaction_record.end_contract_state_tree_root,
                                F::from_canonical_u64(sig_contract_id.unwrap_or_default()),
                                F::from_canonical_u64(user_session_mgr.current_checkpoint_id),
                            );

                            tracing::info!(
                                "user_contract_state: {}",
                                serde_json::to_string_pretty(&user_contract_state)?
                            );
                            let proof_tree_root = user_session_mgr
                                .mgr
                                .proof_tree_state
                                .get_proof_tree_root()
                                .await;
                            user_session_mgr
                                .mgr
                                .lps
                                .set_proof_tree_root(proof_tree_root);
                            let user_contract_state_reader = StateReader::new(
                                user_contract_state,
                                user_session_mgr.mgr.lps.cmd_store.clone(),
                                user_session_mgr.mgr.lps.state_tree_store.clone(),
                            )
                            .await;

                            SoftwareDefinedSignatureWitnessInput::PLONKY2(
                                PSoftwareDefinedSignatureWitnessInput {
                                    state_reader: user_contract_state_reader,
                                    circuit_inputs: cfc_call_inputs,
                                },
                            )
                        }
                    };

                    let private_key = self
                        .wallet
                        .software_defined_public_key_to_private_key_store
                        .get(&pk_info.public_key_param)
                        .ok_or(anyhow::format_err!(
                            "public key `{}` does not exist in the store",
                            pk_info.public_key_param.to_string()
                        ))?;

                    sdc.prove(*private_key, &input, sighash).await?
                } else {
                    return anyhow::bail!("software defined sign need fingerprint");
                }
            }
        };

        user_session_mgr
            .mgr
            .proof_tree_state
            .finalize_tree(self.wallet.random_circuit_manager())
            .await?;

        let public_key_param = self
            .wallet_keys_store
            .get(&public_key)
            .ok_or(anyhow::format_err!(
                "user {} not found, cannot get public key param",
                user_session_mgr.user_id
            ))?
            .public_key_param;

        let (circuit_fingerprint, circuit_verifier_config) = match sign_type {
            SignType::ZKSign => (
                self.wallet.random_circuit_manager().zk_circuit_fingerprint().await?,
                self.wallet
                    .random_circuit_manager()
                    .zk_circuit_verifier_config()
                    .await?,
            ),
            SignType::SECP256K1Sign => (
                self.wallet
                    .random_circuit_manager()
                    .secp_circuit_fingerprint()
                    .await?,
                self.wallet
                    .random_circuit_manager()
                    .secp_circuit_verifier_config()
                    .await?,
            ),
            SignType::SoftwareDefinedSign => {
                let fingerprint = fingerprint.ok_or(anyhow::format_err!(
                    "software defined sign need fingerprint"
                ))?;
                let sdc = self
                    .wallet
                    .software_defined_circuits
                    .get(&fingerprint)
                    .ok_or(anyhow::format_err!(
                        "software defined circuit `{}` not found",
                        fingerprint.to_string()
                    ))?;
                (
                    sdc.get_fingerprint(),
                    sdc.get_verifier_config_ref().to_owned(),
                )
            }
        };

        tracing::info!(
            "prove end cap with network magic {:x}, nonce {}, fingerprint {}, public key param {}, signature proof {:?}",
            QED_NETWORK_MAGIC_REGTEST,
            user_session_mgr.nonce,
            circuit_fingerprint,
            public_key_param,
            signature_proof.public_inputs
        );
        let nonce = user_session_mgr.nonce.clone();
        let end_cap_proof = user_session_mgr
            .mgr
            .prove_end_cap(
                &self.wallet.random_circuit_manager(),
                QED_NETWORK_MAGIC_REGTEST,
                nonce,
                circuit_fingerprint,
                public_key_param,
                signature_proof,
                circuit_verifier_config,
            )
            .await?;

        let user_ec_input = user_session_mgr.mgr.get_api_input().await?;
        tracing::info!(
            "get user ec input: {}",
            serde_json::to_string_pretty(&user_ec_input)?
        );
        let req = QSubmitEndCapRPCRequest {
            user_ec_input,
            proof: end_cap_proof,
        };

        user_session_mgr
            .rpc_provider
            .submit_end_cap_proof::<F>(req)
            .await?;

        // update nonce
        user_session_mgr.nonce = nonce + F::from_noncanonical_u64(1);

        Ok(())
    }

    pub fn get_deploy_contract_cmd(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> anyhow::Result<QBCDeployContract<F>> {
        let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT as usize;

        let (_result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions::<C, D>(
            deployer,
            contract_state_tree_height as u8,
            &circuit_defs,
        )?;
        Ok(deploy_cmd)
    }

    pub async fn deploy_contract(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> anyhow::Result<()> {
        let deploy_cmd = self.get_deploy_contract_cmd(deployer, circuit_defs)?;

        self.st_provider
            .deploy_contract::<F>(QDeployContractRPCRequest {
                deploy_contract: deploy_cmd,
            })
            .await?;

        Ok(())
    }

    pub async fn get_claim_rewards_call_args(
        &self,
        public_key: QHashOut<F>,
        proofs_with_checkpoints: Vec<(u64, Vec<JobInfo>)>
    ) -> anyhow::Result<Vec<ContractCallArgs>> {
        let mut checkpoint_jobs: HashMap<u64, Vec<(JobInfo, VariableHeightRewardMerkleProof)>> = HashMap::new();

        for (checkpoint_id, mut job_infos) in proofs_with_checkpoints {

            job_infos.retain(|job_info| {
                matches!(job_info.job_id.circuit_type,
                    ProvingJobCircuitType::GUTAOnlyRegisterUsers
                    | ProvingJobCircuitType::GUTARegisterUsers
                    | ProvingJobCircuitType::GUTATwoEndCap
                    | ProvingJobCircuitType::GUTATwoGUTA
                    | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
                    | ProvingJobCircuitType::GUTALeftGUTARightEndCap
                    | ProvingJobCircuitType::GUTASingleEndCap
                    | ProvingJobCircuitType::GUTAVerifyToCap
                    | ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade
                    | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade
                    | ProvingJobCircuitType::GUTANoChange
                )
            });

            if job_infos.is_empty() {
                continue;
            }

            tracing::info!("Checkpoint {} - Found {} valid jobs", checkpoint_id, job_infos.len());

            for job_info in job_infos {

                match get_job_proof(&self.st_provider, &job_info, checkpoint_id).await {
                    Ok((actual_checkpoint_id, job_proof)) => {
                        tracing::info!("Found job proof for checkpoint {}, job {:?}", actual_checkpoint_id, job_info.job_id);
                        checkpoint_jobs.entry(actual_checkpoint_id)
                            .or_insert_with(Vec::new)
                            .push((job_info, job_proof));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get job proof for checkpoint {}, job {:?}: {}", checkpoint_id, job_info.job_id, e);
                    }
                }
            }
        }

        if checkpoint_jobs.is_empty() {
            tracing::info!("No valid checkpoints with rewards to claim");
            return Ok(Vec::new());
        }

        let mut sorted_checkpoints: Vec<_> = checkpoint_jobs.keys().copied().collect();
        sorted_checkpoints.sort();

        let mut all_proofs_with_checkpoints = Vec::new();

        for &checkpoint_id in &sorted_checkpoints {
            let jobs = checkpoint_jobs.get(&checkpoint_id).unwrap();

            let checkpoint_leaf = self.st_provider.get_checkpoint_leaf_data(checkpoint_id).await?;
            let fees_collected = checkpoint_leaf.stats.fees_collected.to_canonical_u64();
            let gutas_completed = checkpoint_leaf.stats.pm_jobs_completed.gutas_completed.to_canonical_u64();

            let proposed_reward = if gutas_completed > 0 { fees_collected / gutas_completed } else { 0u64 };

            if proposed_reward == 0 {
                tracing::warn!("Skipping checkpoint {} due to zero reward (fees_collected={}, gutas_completed={})",
                    checkpoint_id, fees_collected, gutas_completed);
                continue;
            }

            tracing::info!("Checkpoint {} - Reward: {}, Jobs: {}", checkpoint_id, proposed_reward, jobs.len());
            for (job_info, _) in jobs {
                tracing::info!("  - {} ({})", job_info.job_id.to_hex_string(), match &job_info.location {
                    JobLocation::Coordinator => "coordinator".to_string(),
                    JobLocation::Realm(id) => format!("realm:{}", id),
                });
            }

            for (_, proof) in jobs {
                all_proofs_with_checkpoints.push(ProofWithCheckpoint {
                    checkpoint_id,
                    proof: proof.clone(),
                    proposed_reward,
                });
            }
        }

        if all_proofs_with_checkpoints.is_empty() {
            tracing::info!("No checkpoints with valid rewards to claim");
            return Ok(Vec::new());
        }

        let mut all_contract_calls = build_claim_calls_for_multi_checkpoints(&all_proofs_with_checkpoints).await;

        if all_contract_calls.is_empty() {
            tracing::info!("No checkpoints with valid rewards to claim");
            return Ok(Vec::new());
        }

        let last_checkpoint = all_proofs_with_checkpoints
            .last()
            .unwrap()
            .checkpoint_id;

        all_contract_calls.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "end_session".to_string(),
            inputs: vec![last_checkpoint],
        });

        all_contract_calls.push(ContractCallArgs {
            contract_id: TOKEN_CONTRACT_ID as u64,
            method_name: "simple_claim_pow_rewards".to_string(),
            inputs: vec![last_checkpoint],
        });

        if all_contract_calls.is_empty() {
            tracing::info!("No rewards to claim");
            return Ok(Vec::new());
        }

        tracing::info!("Executing {} contract calls in single transaction", all_contract_calls.len());
        Ok(all_contract_calls)
    }

    fn add_claim_call_for_batch(
        &self,
        contract_call_args: &mut Vec<ContractCallArgs>,
        mining_rewards_contract_id: u64,
        batch: Vec<(u64, VariableHeightRewardMerkleProof, u64)>,
        method_name: &str,
    ) -> anyhow::Result<()> {
        let batch_size = batch.len();
        let mut batch_inputs = Vec::new();

        for (checkpoint_id, _, _) in &batch {
            batch_inputs.push(*checkpoint_id);
        }

        for (_, proof, _) in &batch {
            self.serialize_proof_to_inputs(proof, &mut batch_inputs);
        }

        for (_, _, proposed_reward) in &batch {
            batch_inputs.push(*proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: mining_rewards_contract_id,
            method_name: method_name.to_string(),
            inputs: batch_inputs,
        });

        tracing::info!("Added {} call with {} proofs", method_name, batch_size);
        Ok(())
    }

    fn serialize_proof_to_inputs(&self, proof: &VariableHeightRewardMerkleProof, inputs: &mut Vec<u64>) {
        for j in 0..qed_core::job::id::GUTA_REWARDS_TREE_MAX_HEIGHT {
            if j < proof.top_siblings.len() {
                let sibling = &proof.top_siblings[j];
                inputs.extend(vec![
                    sibling.sibling_branch.0.elements[0].0,
                    sibling.sibling_branch.0.elements[1].0,
                    sibling.sibling_branch.0.elements[2].0,
                    sibling.sibling_branch.0.elements[3].0,
                    sibling.sibling_reward_leaf.0.elements[0].0,
                    sibling.sibling_reward_leaf.0.elements[1].0,
                    sibling.sibling_reward_leaf.0.elements[2].0,
                    sibling.sibling_reward_leaf.0.elements[3].0,
                ]);
            } else {
                inputs.extend(vec![0u64; 8]);
            }
        }
        inputs.extend(vec![
            proof.sibling_branch.0.elements[0].0,
            proof.sibling_branch.0.elements[1].0,
            proof.sibling_branch.0.elements[2].0,
            proof.sibling_branch.0.elements[3].0,
        ]);
        inputs.extend(vec![
            proof.reward_leaf.0.elements[0].0,
            proof.reward_leaf.0.elements[1].0,
            proof.reward_leaf.0.elements[2].0,
            proof.reward_leaf.0.elements[3].0,
        ]);
        inputs.extend(vec![proof.proof_height.0, proof.index.0]);
    }

    pub async fn claim_rewards(
        &self,
        user_pk_hash: QHashOut<F>,
        proofs_with_checkpoints: Vec<(u64, Vec<JobInfo>)>,
    ) -> anyhow::Result<()> {
        self.claim_rewards_with_sign_type(
            user_pk_hash,
            proofs_with_checkpoints,
            SignType::SECP256K1Sign,
            None,
            None,
            vec![],
        ).await?;
        Ok(())
    }

    pub async fn claim_rewards_with_sign_type(
        &self,
        user_pk_hash: QHashOut<F>,
        proofs_with_checkpoints: Vec<(u64, Vec<JobInfo>)>,
        sign_type: SignType,
        fingerprint: Option<QHashOut<F>>,
        sig_contract_id: Option<u64>,
        sign_inputs: Vec<u64>,
    ) -> anyhow::Result<()> {
        let contract_call_args = self.get_claim_rewards_call_args(
            user_pk_hash,
            proofs_with_checkpoints,
        ).await?;

        self.exec_contract_call_with_sign_type(
            user_pk_hash,
            contract_call_args,
            sign_type,
            fingerprint,
            sig_contract_id,
            sign_inputs,
        ).await?;
        Ok(())
    }

    pub async fn get_zk_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        self.wallet.get_zk_pk_info(private_key).await
    }

    pub async fn get_secp_public_key(
        &self,
        private_key: QHashOut<F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        self.wallet.get_secp_pk_info(private_key).await
    }

    pub async fn get_random_keypair(&self) -> anyhow::Result<WalletKeyPair> {
        let private_key = QHashOut::<F>::rand();
        let pk_info = self.get_zk_public_key(private_key).await?;
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

// #[cfg(feature = "is_sync")]
pub async fn run(args: WalletSessionArgs) -> anyhow::Result<()> {
    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::<F>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let contract_call_args: Vec<ContractCallArgs> =
        serde_json::from_str(&std::fs::read_to_string(args.contract_calls)?)?;

    let mut wallet_session = WalletSession::new(&rpc_config).await?;
    let public_key = wallet_session.add_user(private_key).await?;

    wallet_session.exec_contract_call(public_key, contract_call_args).await?;

    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_scenario0() -> anyhow::Result<()> {
        qed_common::setup_logging()?;
        tracing::info!("test_scenario0");
        let project_path = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| anyhow::format_err!("Error `{}`, cannot get CARGO_MANIFEST_DIR env", e))?;

        let private_key0 = QHashOut::<GoldilocksField>::from_str(
            "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",
        )?;
        let private_key1 = QHashOut::<GoldilocksField>::from_str(
            "f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d",
        )?;

        let config_str = std::fs::read_to_string(Path::new(&project_path).join("../config.json"))?;
        let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
        let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;

        let circuit_defs =
            serde_json::from_str::<Vec<DPNFunctionCircuitDefinition>>(&std::fs::read_to_string(
                Path::new(&project_path).join("../examples/target/examples.json"),
            )?)?;

        let mut wallet_session = super::WalletSession::new(&rpc_config)?;

        let deployer_pk_info = wallet_session.get_zk_public_key(private_key0)?;
        wallet_session.deploy_contract(deployer_pk_info.qfhash::<QEDHasher>(), circuit_defs)?;

        let user0 = wallet_session.register_user(private_key0)?;
        let user1 = wallet_session.register_user(private_key0)?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // add user0
        wallet_session.add_user(private_key0)?;

        // add user1
        wallet_session.add_user(private_key1)?;

        // user0 mint 1000
        wallet_session.exec_contract_call(
            user0,
            vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_mint".to_string(),
                inputs: vec![1000],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // user0 transfer 500 to user1
        wallet_session.exec_contract_call(
            user0,
            vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![1, 500],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // user1 claim
        wallet_session.exec_contract_call(
            user1,
            vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_claim".to_string(),
                inputs: vec![0],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // user1 transfer 500 to user0
        wallet_session.exec_contract_call(
            user1,
            vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![0, 500],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        Ok(())
    }

    #[test]
    fn test_two_contracts() -> anyhow::Result<()> {
        qed_common::setup_logging()?;
        tracing::info!("test_two_contracts");
        let project_path = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|e| anyhow::format_err!("Error `{}`, cannot get CARGO_MANIFEST_DIR env", e))?;

        let private_key0 = QHashOut::<GoldilocksField>::from_str(
            "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",
        )?;
        let private_key1 = QHashOut::<GoldilocksField>::from_str(
            "f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d",
        )?;

        let config_str = std::fs::read_to_string(Path::new(&project_path).join("../config.json"))?;
        let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
        let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;

        let circuit_defs =
            serde_json::from_str::<Vec<DPNFunctionCircuitDefinition>>(&std::fs::read_to_string(
                Path::new(&project_path).join("../examples/target/examples.json"),
            )?)?;

        let mut wallet_session = super::WalletSession::new(&rpc_config)?;

        let deployer_pk_info = wallet_session.get_zk_public_key(private_key0)?;
        wallet_session
            .deploy_contract(deployer_pk_info.qfhash::<QEDHasher>(), circuit_defs.clone())?;
        wallet_session.deploy_contract(deployer_pk_info.qfhash::<QEDHasher>(), circuit_defs)?;

        let user0 = wallet_session.register_user(private_key0)?;
        let user1 = wallet_session.register_user(private_key0)?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // add user0
        wallet_session.add_user(private_key0)?;

        // add user1
        wallet_session.add_user(private_key1)?;

        // user0 mint 1000 contract 0
        wallet_session.exec_contract_call(
            user0,
            vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_mint".to_string(),
                inputs: vec![1000],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        // user0 mint 1000 contract 1
        wallet_session.exec_contract_call(
            user0,
            vec![ContractCallArgs {
                contract_id: 1,
                method_name: "simple_mint".to_string(),
                inputs: vec![1000],
            }],
        )?;

        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));
        // wallet_session.st_provider.produce_block::<F>()?;
        thread::sleep(Duration::from_secs(20));

        Ok(())
    }
}
