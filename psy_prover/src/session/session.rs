use std::{collections::HashMap, str::FromStr};

use dashmap::DashMap;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::{
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::{
    args::{ContractCallArgs, ContractCallData, DPNSoftwareDefinedCallData, SignType, WalletSessionArgs},
    data::qhashout::QHashOut,
    job::id::{ProvingJobCircuitType, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
    traits::to_qfelts::ToQFelts,
    ups::circuits::LocalCircuitType,
    JobInfo, JobLocation,
};
use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use psy_config::{
    network_constants::{CONTRACT_FUNCTION_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT, TOKEN_CONTRACT_ID, UPS_SESSION_PROOF_TREE_HEIGHT},
    MINING_REWARDS_CONTRACT_ID, PSY_NETWORK_MAGIC,
};
use psy_crypto::{
    common::user_id::get_registration_id_from_user_id,
    hash::traits::{
        hasher::{MerkleZeroHasher, MerkleZeroHasherWithMarkedLeaf},
        qhashable::QFieldHashable,
    },
    signature::zk::data::ZKPublicKeyInfo,
};
use psy_data::{
    config::store_config::{PsyHash, PsyHasher},
    guta::end_cap_input::SubmitUserEndCapNonProofInput,
    qblock::cmds::deploy_contract::{get_code_root_by_code_hashes, QBCDeployContract, QContractABI},
    qdata::{checkpoint::PsyBlockState, contract::ContractCodeDefinition, user::PsyUserLeaf, user_contract_state::UserContractState},
    qstore::{
        controllers::{
            proving_session::{PsyLocalProvingSessionStore, PsyReadLocalProvingSessionStore},
            session_info::SessionCircuitInfoStore,
        },
        imm::{
            cmd::{
                QSRCmdGetContractCodeDefinition, QSRCmdGetUserLeafData, QSRHashCmd, QSRHashCmdGetUserContractStateTreeRoot, QSRMerkleCmd,
                QSRMerkleCmdGetUserTreeMerkleProof,
            },
            cmd_processor::{PsyReadCommandProcessorSync, PsyReadCommandProcessorSyncMut},
        },
    },
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync,
        qtreedata::{PsyComboDataStoreReaderSync, QTreeDataStoreReaderSync},
    },
};
use psy_dpn_circuit::circuits::cfc::DapenContractFunctionCircuit;
pub use psy_provider::session::TxStatus;
use psy_provider::{
    provider::{NetworkConfig, ProveProxyRpcProvider, QUserRpcProvider, RpcProvider},
    request::{DPNSoftwareDefinedSignatureInput, QDeployContractRPCRequest, QRegisterUserRPCRequest, QSubmitEndCapRPCRequest},
};
use psy_ups_circuit::{
    circuit_manager::core::PsyUPSStepCircuitManager,
    session::UserProvingSessionManager,
    signature::software_defined::{DPNSoftwareDefinedSignatureGadget, Plonky2SoftwareDefinedSignatureGadget},
};
use psy_vm::{
    dpn::{
        contract::{cfc_code_definition_to_dapen_fc, dapen_fc_to_cfc_code_definition, hash_dpn_function},
        vm::def::DPNFunctionCircuitDefinition,
    },
    ups::{
        circuit_manager::{PortableQTreeRecursion, UPSCircuitManager},
        signature::Plonky2SoftwareDefinedSignatureInput,
        state_reader::StateReader,
    },
};
use serde::{Deserialize, Serialize};
use tracing::warn;

type UserEndCapUUID = String;

use crate::{
    session::{build_claim_calls_for_multi_checkpoints, ProofWithCheckpoint},
    signature::{
        context::SignContext,
        traits::{SignatureCircuitInfo, SignatureResult},
    },
    wallet::memory_wallet::PsyMemoryWallet,
};

trait UPSWithTreeRecursion<C: GenericConfig<D>, const D: usize>: UPSCircuitManager<C, D> + PortableQTreeRecursion<C, D> + Send + Sync
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasherWithMarkedLeaf<HashOut<C::F>> + MerkleZeroHasherWithMarkedLeaf<QHashOut<C::F>>,
{
}

impl<T, C: GenericConfig<D>, const D: usize> UPSWithTreeRecursion<C, D> for T
where
    T: UPSCircuitManager<C, D> + PortableQTreeRecursion<C, D> + Send + Sync,
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasherWithMarkedLeaf<HashOut<C::F>> + MerkleZeroHasherWithMarkedLeaf<QHashOut<C::F>>,
{
}

pub fn gen_contract_deploy_and_circuits_for_functions<C: GenericConfig<D>, const D: usize>(
    deployer: QHashOut<C::F>,
    contract_state_tree_height: u8,
    defs: &[DPNFunctionCircuitDefinition],
) -> anyhow::Result<(Vec<DapenContractFunctionCircuit<C, D>>, QBCDeployContract<C::F>)>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasherWithMarkedLeaf<QHashOut<C::F>>,
{
    let code_defs = defs.iter().map(|x| dapen_fc_to_cfc_code_definition(x)).collect::<Vec<_>>();
    let mut whitelist_leaves = Vec::with_capacity(defs.len() * 2);
    let mut code_hashes = Vec::with_capacity(defs.len());
    let circuits = defs
        .iter()
        .map(|x| {
            let c = DapenContractFunctionCircuit::<C, D>::new(x, contract_state_tree_height as usize, UPS_SESSION_PROOF_TREE_HEIGHT as usize, false);
            whitelist_leaves.push(c.get_fingerprint());

            let inputs_outputs_combo = ((x.circuit_outputs.len() as u64) << 32u64) | (x.circuit_inputs.len() as u64);
            whitelist_leaves.push(QHashOut::from_values(x.method_id as u64, inputs_outputs_combo, 0, 0));
            let code_hash = hash_dpn_function::<C::F>(x);
            code_hashes.push(code_hash);
            c
        })
        .collect::<Vec<_>>();

    let deploy = QBCDeployContract {
        deployer,
        code_definition: ContractCodeDefinition {
            state_tree_height: contract_state_tree_height as u16,
            functions: code_defs,
        },
        function_whitelist: whitelist_leaves,
        code_root: get_code_root_by_code_hashes::<C::F, C::Hasher>(&code_hashes, CONTRACT_FUNCTION_TREE_HEIGHT - 1),
    };

    Ok((circuits, deploy))
}

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub async fn prove_func<R, CM: UPSCircuitManager<C, D> + ?Sized>(
    contract_code: ContractCodeDefinition,
    circuit_mgr: &CM,
    mgr: &mut UserProvingSessionManager<F, PoseidonHash, R, C, D>,
    contract_id: u64,
    fn_name: &str,
    inputs: Vec<F>,
) -> anyhow::Result<()>
where
    R: PsyReadCommandProcessorSync<F> + PsyComboDataStoreReaderSync<F> + psy_data::qstore::imm::cmd_processor::QUserIdManager + Send + Sync,
{
    circuit_mgr.register_contract_circuits(contract_id, &contract_code).await?;

    let (fn_id, dapen_fc) = circuit_mgr
        .resolve_contract_function_by_method_name(contract_id, &contract_code, fn_name.to_string())
        .await?;

    mgr.prove_standard_call(circuit_mgr, F::from_canonical_u64(contract_id), fn_id as u32, &dapen_fc, inputs)
        .await
}

pub struct WalletSession {
    pub wallet: PsyMemoryWallet,
    pub circuit_info: SessionCircuitInfoStore<F>,
    pub st_provider: RpcProvider,

    pub user_session_mgrs: DashMap<QHashOut<F>, UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl WalletSession {
    async fn check_user_state(&self, user_id: u64, nonce: F) -> anyhow::Result<()> {
        match self
            .st_provider
            .with_user_id_owned(user_id)
            .get_tx_status(user_id, nonce.to_noncanonical_u64())
            .await?
        {
            TxStatus::Confirmed => {
                tracing::warn!("tx status is confirmed");
                Err(anyhow::format_err!(
                    "another similar tx is confirmed while building this tx, please rebuild the tx later"
                ))
            }
            TxStatus::Pending => {
                tracing::warn!("tx status is pending");
                Err(anyhow::format_err!("another similar tx is pending, please wait for it to be confirmed"))
            }
            TxStatus::Submittable => {
                tracing::debug!("tx status is submittable");
                Ok(())
            }
        }
    }

    pub async fn check_tx_is_confirmed(&mut self, checkpoint_id: u64, user_id: u64, tx_hash: QHashOut<F>) -> anyhow::Result<bool> {
        let user_leaf_data = self
            .st_provider
            .with_user_id_owned(user_id)
            .get_user_leaf_data(checkpoint_id, user_id)
            .await?;
        Ok(user_leaf_data.qfhash::<PsyHasher>() == tx_hash)
    }

    pub async fn new(rpc_config: &psy_config::NetworkConfigGoldilocks) -> anyhow::Result<Self> {
        tracing::info!("init rpc provider");
        let st_provider = RpcProvider::new_with_config(rpc_config)?;

        tracing::info!("init wallet");
        tracing::info!("init ups step circuit manager");
        let mut main_circuits: Vec<Box<dyn UPSCircuitManager<C, D> + Send + Sync>> = Vec::new();

        for proxy_url in rpc_config.prove_proxy_url.iter() {
            match ProveProxyRpcProvider::new_with_config(proxy_url.to_string()).await {
                Ok(main_circuit) => main_circuits.push(Box::new(main_circuit)),
                Err(e) => {
                    tracing::warn!("prove proxy url `{}` is invalid, skip: {}", proxy_url, e);
                }
            }
        }
        if main_circuits.is_empty() {
            tracing::warn!("no valid prove proxy url, use local circuit manager");
            main_circuits.push(Box::new(PsyUPSStepCircuitManager::<C, D>::new_with_config(PSY_NETWORK_MAGIC)));
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
            main_circuit.as_ref().register_info(&mut circuit_info).await;
        }

        let wallet = PsyMemoryWallet::new(main_circuits);

        Ok(WalletSession {
            wallet,
            circuit_info,
            st_provider,
            user_session_mgrs: DashMap::new(),
        })
    }

    pub async fn register_user(&mut self, private_key: QHashOut<F>, fingerprint: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        let pk_info = self.wallet.get_or_create_user(private_key, fingerprint).await?;
        let public_key = pk_info.qfhash::<PsyHasher>();

        if let Ok(user_ids) = self.st_provider.get_user_ids_for_public_key(public_key).await {
            tracing::info!("user `{}` already registered with id {}", public_key, user_ids[0]);
            return Ok(public_key);
        }

        self.st_provider.register_user(QRegisterUserRPCRequest { public_key: pk_info }).await?;

        tracing::info!("user `{}` registered", public_key);
        tracing::warn!("please add this user after 2 checkpoints!");
        Ok(public_key)
    }

    pub async fn add_user(&mut self, private_key: QHashOut<F>, fingerprint: QHashOut<F>) -> anyhow::Result<QHashOut<F>> {
        let pk_info = self.wallet.get_or_create_user(private_key, fingerprint).await?;
        let public_key = pk_info.qfhash::<PsyHasher>();

        let user_id = self
            .st_provider
            .get_user_ids_for_public_key(public_key)
            .await
            .map_err(|e| anyhow::format_err!("User {} not registered. Please register first: {}", public_key, e))?[0];
        self.update_circuit_mgr(public_key).await?;
        tracing::info!("user {} with id {} added", public_key.to_string(), user_id);
        Ok(public_key)
    }

    pub async fn update_circuit_mgr(&self, public_key: QHashOut<F>) -> anyhow::Result<()> {
        let user_id = self
            .st_provider
            .get_user_ids_for_public_key(public_key)
            .await
            .map_err(|e| anyhow::format_err!("User {} not registered. Please register first: {}", public_key, e))?[0];
        if let Some((_, existing_mgr)) = self.user_session_mgrs.remove(&public_key) {
            let cleaned_mgr = existing_mgr.into_clean_for_user(F::from_canonical_u64(user_id)).await?;
            self.user_session_mgrs.insert(public_key, cleaned_mgr);
        } else {
            let rpc_provider = self.st_provider.with_user_id_owned(user_id);
            let lps = PsyLocalProvingSessionStore::new_at(
                rpc_provider,
                F::ZERO,
                F::from_canonical_u64(user_id),
                F::ZERO,
                F::ZERO,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
            )
            .into_clean_for_user(F::from_canonical_u64(user_id))
            .await?;

            let circuit_mgr = self.wallet.random_circuit_manager();
            let mgr =
                UserProvingSessionManager::<F, _, _, C, D>::new(lps, self.circuit_info.clone(), circuit_mgr.ups_circuit_whitelist_root().await?)
                    .await?;

            self.user_session_mgrs.insert(public_key, mgr);
        }
        Ok(())
    }

    pub async fn exec_contract_call(&self, public_key: QHashOut<F>, call_data: ContractCallData) -> anyhow::Result<UserEndCapUUID> {
        if call_data.contract_calls.is_empty() {
            anyhow::bail!("No contract calls to execute");
        }

        tracing::info!("exec contract call: {}", serde_json::to_string_pretty(&call_data.contract_calls)?);
        let pk_info = self.wallet.get_public_key_info(&public_key).await?;
        tracing::info!(
            "exec contract call for fingerprint {} (sign data provided: {})",
            pk_info.fingerprint,
            call_data.software_defined_call.is_some()
        );
        let result = self.st_provider.get_latest_block_state().await?;
        tracing::info!("start session on global checkpoint: {}", result.checkpoint_id);
        self.start_session(public_key).await?;
        tracing::info!("prove contract calls");
        self.prove_contract_call(public_key, call_data.contract_calls).await?;
        tracing::info!("sign and submit on global checkpoint: {}", result.checkpoint_id);
        let user_end_cap_uuid = self.sign_and_submit(public_key, call_data.software_defined_call).await?;
        Ok(user_end_cap_uuid)
    }

    pub async fn start_session(&self, public_key: QHashOut<F>) -> anyhow::Result<()> {
        self.update_circuit_mgr(public_key).await?;
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;

        let latest_block_state = user_session_mgr.lps.get_read_store().get_latest_block_state().await?;
        let global_latest_block_state = self.st_provider.get_coordinator_latest_block_state().await?;

        if latest_block_state.checkpoint_id <= global_latest_block_state.checkpoint_id {
            tracing::info!(
                "block state: realm latest checkpoint {}, coordinator checkpoint {}",
                latest_block_state.checkpoint_id,
                global_latest_block_state.checkpoint_id
            );
        } else {
            tracing::error!(
                "realm latest checkpoint {} is ahead coordinator checkpoint {}",
                latest_block_state.checkpoint_id,
                global_latest_block_state.checkpoint_id
            );
            return Err(anyhow::format_err!(
                "realm latest checkpoint {} is ahead of coordinator checkpoint {}",
                latest_block_state.checkpoint_id,
                global_latest_block_state.checkpoint_id
            ));
        };

        tracing::info!("local proving ups start");
        tracing::info!("user session manager nonce: {}", user_session_mgr.lps.get_nonce());

        user_session_mgr.prove_ups_start(self.wallet.random_circuit_manager().as_ref()).await?;

        let user_id = user_session_mgr.lps.get_current_user_id_64();
        let registration_id = get_registration_id_from_user_id(user_id);
        let checkpoint = user_session_mgr.lps.get_current_start_checkpoint_id_u64();

        tracing::info!(
            "check if user {}: {} is registered at checkpoint {}, registration_id: {}",
            user_id,
            public_key.to_string(),
            checkpoint,
            registration_id
        );
        let registration_leaf_hash = self
            .st_provider
            .with_user_id_owned(user_id)
            .get_user_registration_tree_leaf_hash(checkpoint, registration_id)
            .await?;

        if registration_leaf_hash == QHashOut::ZERO {
            anyhow::bail!(
                "user {}: {} of registration id {} is not registered at checkpoint {}, please check it first",
                user_id,
                public_key.to_string(),
                registration_id,
                checkpoint
            );
        }

        let nonce = user_session_mgr.lps.get_nonce();
        self.check_user_state(user_id, nonce).await?;

        Ok(())
    }

    pub async fn prove_contract_call(&self, public_key: QHashOut<F>, contract_call_args: Vec<ContractCallArgs>) -> anyhow::Result<()> {
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;
        let cmd_store = user_session_mgr.lps.get_cmd_store();
        for contract_call_arg in contract_call_args {
            tracing::info!(
                "prove contract call at contract {}, method {}",
                contract_call_arg.contract_id,
                contract_call_arg.method_name
            );
            let contract_code = user_session_mgr
                .lps
                .resolve_get_contract_code_mut(&QSRCmdGetContractCodeDefinition {
                    contract_id: contract_call_arg.contract_id,
                })
                .await?;
            prove_func(
                contract_code,
                self.wallet.random_circuit_manager().as_ref(),
                &mut *user_session_mgr,
                contract_call_arg.contract_id,
                &contract_call_arg.method_name,
                contract_call_arg.inputs.iter().map(|x| F::from_noncanonical_u64(*x)).collect(),
            )
            .await?;
        }
        user_session_mgr.prove_burn_fee(self.wallet.random_circuit_manager().as_ref()).await?;

        let user_id = user_session_mgr.lps.get_current_user_id_64();
        let nonce = user_session_mgr.lps.get_nonce();
        self.check_user_state(user_id, nonce).await?;

        Ok(())
    }

    pub async fn sign(
        &self,
        public_key: QHashOut<F>,
        software_defined_call: Option<DPNSoftwareDefinedCallData>,
    ) -> anyhow::Result<(SubmitUserEndCapNonProofInput<F>, ProofWithPublicInputs<F, C, D>)> {
        let pk_info = self.wallet.get_public_key_info(&public_key).await?;
        let mut user_session_mgr = self
            .user_session_mgrs
            .get_mut(&public_key)
            .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?;

        tracing::info!(
            "sign and submit for fingerprint {} (software defined call provided: {})",
            pk_info.fingerprint,
            software_defined_call.is_some()
        );
        let mut sign_context = SignContext::new(pk_info.fingerprint);

        if let Some(ref data) = software_defined_call {
            if self.wallet.has_psy_software_defined_circuit(&pk_info.fingerprint) {
                sign_context = self
                    .build_psy_software_defined_context(data, pk_info.fingerprint, &mut user_session_mgr, sign_context)
                    .await?;
            } else if self.wallet.has_plonky2_software_defined_circuit(&pk_info.fingerprint) {
                sign_context = self
                    .build_plonky2_software_defined_context(data, pk_info.fingerprint, &mut user_session_mgr)
                    .await?;
            } else {
                return Err(anyhow::format_err!("Software defined circuit `{}` not found", pk_info.fingerprint));
            };
        }

        let nonce = user_session_mgr.lps.get_nonce();
        let sighash = user_session_mgr.get_sighash(PSY_NETWORK_MAGIC, nonce);

        tracing::info!("zk sign for signhash: {}, nonce: {}", sighash.to_string(), nonce);
        let signature_result = self.wallet.sign_with_public_key(&public_key, &sign_context, sighash).await?;
        let SignatureResult {
            proof: signature_proof,
            circuit_info,
        } = signature_result;

        user_session_mgr
            .proof_tree_state
            .finalize_tree(self.wallet.random_circuit_manager().as_ref())
            .await?;

        let public_key_param = pk_info.public_key_param;

        let SignatureCircuitInfo {
            circuit_fingerprint,
            verifier_config: circuit_verifier_config,
        } = circuit_info;

        tracing::info!(
            "prove end cap with network magic {:x}, nonce {}, fingerprint {}, public key param {}, signature proof {:?}",
            PSY_NETWORK_MAGIC,
            nonce,
            circuit_fingerprint,
            public_key_param,
            signature_proof.public_inputs
        );
        let end_cap_proof = user_session_mgr
            .prove_end_cap(
                self.wallet.random_circuit_manager().as_ref(),
                PSY_NETWORK_MAGIC,
                nonce,
                circuit_fingerprint,
                public_key_param,
                signature_proof,
                circuit_verifier_config,
            )
            .await?;

        let user_ec_input = user_session_mgr.get_api_input().await?;
        tracing::info!("get user ec input: {}", serde_json::to_string_pretty(&user_ec_input)?);

        let end_user_leaf_hash = user_ec_input.core.state_transition.end_user_leaf_hash;
        let new_user_leaf = user_ec_input.core.new_user_leaf;
        if end_user_leaf_hash != new_user_leaf.qfhash::<PsyHasher>() {
            anyhow::bail!("end user leaf hash not match");
        }

        let user_id = user_session_mgr.lps.get_current_user_id_64();
        self.check_user_state(user_id, nonce).await?;

        Ok((user_ec_input, end_cap_proof))
    }

    pub async fn sign_and_submit(
        &self,
        public_key: QHashOut<F>,
        software_defined_call: Option<DPNSoftwareDefinedCallData>,
    ) -> anyhow::Result<UserEndCapUUID> {
        let (user_ec_input, end_cap_proof) = self.sign(public_key, software_defined_call).await?;
        let user_id = user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        let req = QSubmitEndCapRPCRequest {
            user_ec_input,
            proof: bincode::serialize(&end_cap_proof)?,
        };

        let tx_hash = req.user_ec_input.get_tx_hash()?;

        let _ = self.st_provider.with_user_id_owned(user_id).submit_end_cap_proof::<F>(req).await?;

        Ok(tx_hash.to_string())
    }

    async fn build_psy_software_defined_context(
        &self,
        call_data: &DPNSoftwareDefinedCallData,
        fingerprint: QHashOut<F>,
        user_session_mgr: &mut UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>,
        mut sign_context: SignContext,
    ) -> anyhow::Result<SignContext> {
        let sdc = self
            .wallet
            .get_psy_software_defined_circuit(&fingerprint)
            .ok_or_else(|| anyhow::format_err!("PSY software defined circuit `{}` not found", fingerprint))?;

        let cfc_call_inputs = call_data.inputs.iter().map(|x| F::from_noncanonical_u64(*x)).collect::<Vec<_>>();

        let cfc_proof_input = user_session_mgr
            .exec_contract_call(F::from_noncanonical_u64(call_data.contract_id), &sdc.fn_def, cfc_call_inputs)
            .await?;

        let signature_input = DPNSoftwareDefinedSignatureInput { cfc_input: cfc_proof_input };

        let current_checkpoint_id = user_session_mgr.lps.get_current_start_checkpoint_id_u64();
        let user_id = user_session_mgr.lps.get_current_user_id_64();
        let start_contract_state_tree_root = user_session_mgr.lps.last_transaction_record().user_contract_tree_update_proof.old_value;
        let checkpoint_tree_root = self.st_provider.get_checkpoint_tree_root(current_checkpoint_id).await?;

        Ok(sign_context.with_psy_signature_input(
            signature_input,
            current_checkpoint_id,
            user_id,
            start_contract_state_tree_root,
            checkpoint_tree_root,
        ))
    }

    async fn build_plonky2_software_defined_context(
        &self,
        call_data: &DPNSoftwareDefinedCallData,
        fingerprint: QHashOut<F>,
        user_session_mgr: &mut UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>,
    ) -> anyhow::Result<SignContext> {
        let user_id = user_session_mgr.lps.get_current_user_id_64();
        let checkpoint_id = user_session_mgr.lps.get_current_start_checkpoint_id_u64();
        let user_leaf = user_session_mgr
            .lps
            .resolve_get_user_leaf_mut(&QSRCmdGetUserLeafData { checkpoint_id, user_id })
            .await?;

        let checkpoint_tree_root = self.st_provider.get_checkpoint_tree_root(checkpoint_id).await?;

        let transaction_record = user_session_mgr.lps.last_transaction_record();

        let circuit_inputs = call_data.inputs.iter().map(|x| F::from_noncanonical_u64(*x)).collect::<Vec<_>>();

        let user_contract_state = UserContractState::new(
            checkpoint_tree_root,
            user_leaf,
            transaction_record.user_contract_tree_update_proof.new_value,
            F::from_canonical_u64(call_data.contract_id),
            F::from_canonical_u64(checkpoint_id),
        );

        let state_reader: StateReader<F, 2, RpcProvider> = StateReader::new(
            user_contract_state,
            user_session_mgr.lps.get_cmd_store().clone(),
            user_session_mgr.lps.get_state_tree_store().clone(),
        )
        .await;

        let plonky2_input = Plonky2SoftwareDefinedSignatureInput {
            state_reader_results: state_reader.to_results(),
            circuit_inputs,
        };

        Ok(SignContext::new(fingerprint)
            .with_contract_id(Some(call_data.contract_id))
            .with_sign_inputs(call_data.inputs.clone())
            .with_plonky2_signature_input(
                plonky2_input,
                checkpoint_id,
                user_id,
                transaction_record.user_contract_tree_update_proof.old_value,
                checkpoint_tree_root,
            ))
    }

    pub fn get_deploy_contract_cmd(
        &self,
        deployer: QHashOut<F>,
        circuit_defs: Vec<DPNFunctionCircuitDefinition>,
    ) -> anyhow::Result<QBCDeployContract<F>> {
        let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT as usize;

        let (_result_circuits, deploy_cmd) =
            gen_contract_deploy_and_circuits_for_functions::<C, D>(deployer, contract_state_tree_height as u8, &circuit_defs)?;
        Ok(deploy_cmd)
    }

    pub async fn deploy_contract(&self, deployer: QHashOut<F>, circuit_defs: Vec<DPNFunctionCircuitDefinition>) -> anyhow::Result<String> {
        let deploy_cmd = self.get_deploy_contract_cmd(deployer, circuit_defs)?;

        let contract_uuid = self
            .st_provider
            .deploy_contract::<F>(QDeployContractRPCRequest { deploy_contract: deploy_cmd })
            .await?;
        Ok(contract_uuid)
    }

    pub async fn get_claim_rewards_call_args(&self, mut job_infos: Vec<JobInfo>) -> anyhow::Result<Vec<ContractCallArgs>> {
        job_infos.retain(|job_info| job_info.job_id.circuit_type.is_guta_job());

        if job_infos.is_empty() {
            tracing::info!("No valid GUTA jobs found after filtering");
            return Ok(Vec::new());
        }

        let mut checkpoint_jobs: HashMap<u64, Vec<VariableHeightRewardMerkleProof>> = HashMap::new();

        match self.st_provider.get_job_proofs(job_infos).await {
            Ok(results) => {
                for (root_job_id, job_proof) in results {
                    let actual_checkpoint_id = root_job_id.goal_id;
                    checkpoint_jobs
                        .entry(actual_checkpoint_id)
                        .or_insert_with(Vec::new)
                        .push(job_proof.pad_to_height(GUTA_REWARDS_TREE_MAX_HEIGHT));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get job proofs: {}", e);
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
            let proofs = checkpoint_jobs.get(&checkpoint_id).unwrap();

            let checkpoint_leaf = self.st_provider.get_checkpoint_leaf_data(checkpoint_id).await?;
            let fees_collected = checkpoint_leaf.stats.fees_collected.to_canonical_u64();
            let gutas_completed = checkpoint_leaf.stats.pm_jobs_completed.gutas_completed.to_canonical_u64();

            let proposed_reward = if gutas_completed > 0 { fees_collected / gutas_completed } else { 0u64 };

            if proposed_reward == 0 {
                tracing::warn!(
                    "Skipping checkpoint {} due to zero reward (fees_collected={}, gutas_completed={})",
                    checkpoint_id,
                    fees_collected,
                    gutas_completed
                );
                continue;
            }

            tracing::info!("Checkpoint {} - Reward: {}, Jobs: {}", checkpoint_id, proposed_reward, proofs.len());
            for proof in proofs {
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

        let last_checkpoint = all_proofs_with_checkpoints.last().unwrap().checkpoint_id;

        all_contract_calls.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID as u64,
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

    pub async fn claim_rewards(&self, user_pk_hash: QHashOut<F>, job_infos: Vec<JobInfo>) -> anyhow::Result<()> {
        let contract_call_args = self.get_claim_rewards_call_args(job_infos).await?;
        self.exec_contract_call(user_pk_hash, ContractCallData::new(contract_call_args)).await?;
        Ok(())
    }

    pub async fn get_zk_public_key(&self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        self.wallet.get_zk_pk_info(private_key).await
    }

    pub async fn get_secp_public_key(&self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
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

#[cfg(all(not(target_arch = "wasm32"), feature = "is_sync"))]
mod tests {
    use std::{path::Path, thread, time::Duration};

    use super::*;

    #[test]
    fn test_scenario0() -> anyhow::Result<()> {
        psy_common::setup_logging()?;
        tracing::info!("test_scenario0");
        let project_path =
            std::env::var("CARGO_MANIFEST_DIR").map_err(|e| anyhow::format_err!("Error `{}`, cannot get CARGO_MANIFEST_DIR env", e))?;

        let private_key0 = QHashOut::<GoldilocksField>::from_str("17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a")?;
        let private_key1 = QHashOut::<GoldilocksField>::from_str("f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d")?;

        let psy_config = psy_config::PsyConfigGoldilocks::from_file(&Path::new(&project_path).join("../../../config.json").to_string_lossy())?;
        let rpc_config = psy_config.get_current_network()?;

        let circuit_defs = serde_json::from_str::<Vec<DPNFunctionCircuitDefinition>>(&std::fs::read_to_string(
            Path::new(&project_path).join("../examples/target/examples.json"),
        )?)?;

        let mut wallet_session = super::WalletSession::new(&rpc_config)?;

        let deployer_pk_info = wallet_session.get_zk_public_key(private_key0)?;
        wallet_session.deploy_contract(deployer_pk_info.qfhash::<PsyHasher>(), circuit_defs)?;

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
        psy_common::setup_logging()?;
        tracing::info!("test_two_contracts");
        let project_path =
            std::env::var("CARGO_MANIFEST_DIR").map_err(|e| anyhow::format_err!("Error `{}`, cannot get CARGO_MANIFEST_DIR env", e))?;

        let private_key0 = QHashOut::<GoldilocksField>::from_str("17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a")?;
        let private_key1 = QHashOut::<GoldilocksField>::from_str("f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d")?;

        let psy_config = psy_config::PsyConfigGoldilocks::from_file(&Path::new(&project_path).join("../../../config.json").to_string_lossy())?;
        let rpc_config = psy_config.get_current_network()?;

        let circuit_defs = serde_json::from_str::<Vec<DPNFunctionCircuitDefinition>>(&std::fs::read_to_string(
            Path::new(&project_path).join("../examples/target/examples.json"),
        )?)?;

        let mut wallet_session = super::WalletSession::new(&rpc_config)?;

        let deployer_pk_info = wallet_session.get_zk_public_key(private_key0)?;
        wallet_session.deploy_contract(deployer_pk_info.qfhash::<PsyHasher>(), circuit_defs.clone())?;
        wallet_session.deploy_contract(deployer_pk_info.qfhash::<PsyHasher>(), circuit_defs)?;

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
