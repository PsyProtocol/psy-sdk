use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use dashmap::DashMap;
use jsonrpsee::{core::async_trait, proc_macros::rpc, types::ErrorObjectOwned};
use k256::ecdsa::signature::hazmat::PrehashSigner;
use plonky2::plonk::{
    config::{GenericConfig, PoseidonGoldilocksConfig},
    proof::ProofWithPublicInputs,
};
use psy_common::data::{alt::AltVerifierOnlyCircuitData, base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::CompressedPublicKey};
use psy_common_circuit::circuits::{
    secp256k1_signature::Secp256K1SignatureCircuit, traits::qstandard::QStandardCircuit, zk_signature::inner,
    zk_signature3::core::PsyBasicZKSignatureCircuit,
};
use psy_config::network_constants::UPS_SESSION_PROOF_TREE_HEIGHT;
use psy_crypto::{
    common::witnesses::qrecursion::{
        header::QRecursionAggStandardHeader,
        proof_data::{QStandardBinaryTreeCircuitType, SimpleQTreeRecursionManagerInclusionProofs},
    },
    hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
    signature,
    signature::{secp256k1, secp256k1::core::PsyCompressedSecp256K1Signature},
};
use psy_data::{
    qdata::contract::ContractCodeDefinition,
    qstore::{
        controllers::session_info::SessionCircuitInfoStore,
        imm::{cmd::QSRCmdGetContractCodeDefinition, cmd_processor::PsyReadCommandProcessorSync},
    },
    ups::{
        start_step::UPSStartStepInput,
        ups_cfc_standard_step::{UPSCFCDeferredTransactionCircuitInput, UPSCFCStandardTransactionCircuitInput},
        ups_end_cap::UPSEndCapFromProofTreeGadgetInput,
    },
};
use psy_dpn_circuit::circuits::cfc::DapenContractFunctionCircuit;
use psy_provider::provider::{NetworkConfig, QCommonCircuitData, RpcProvider};
use psy_provider::request::{
    DPNSoftwareDefinedSignatureInput, QRegisterDPNSoftwareDefinedCircuitRPCRequest, QRegisterPlonky2SoftwareDefinedCircuitRPCRequest,
};
use psy_ups_circuit::{
    circuit_manager::core::PsyUPSStepCircuitManager,
    signature::software_defined::{DPNSoftwareDefinedSignatureGadget, Plonky2SoftwareDefinedSignatureGadget},
};
use psy_vm::{
    dpn::contract::cfc_code_definition_to_dapen_fc, ups::circuit_manager::UPSCircuitManager, vm::cfc_input::DapenContractFunctionCircuitInput,
};
use serde::{Deserialize, Deserializer, Serialize};

type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;
const D: usize = 2;

#[rpc(server, client, namespace = "psy")]
pub trait ProveProxyRpc {
    /// local proving proof generate
    #[method(name = "prove_ups_start")]
    async fn prove_ups_start(&self, input: UPSStartStepInput<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "get_circuits_data")]
    async fn get_circuits_data(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "get_method_id")]
    async fn get_method_id(&self, contract_id: u64, method_name: String) -> Result<u64, ErrorObjectOwned>;

    #[method(name = "get_contract_method_common_data")]
    async fn get_contract_method_common_data(&self, contract_id: u64, method_id: u32) -> Result<QCommonCircuitData<F>, ErrorObjectOwned>;

    #[method(name = "register_contract_circuits")]
    async fn register_contract_circuits(&self, contract_id: u64, contract_code: ContractCodeDefinition) -> Result<(), ErrorObjectOwned>;

    #[method(name = "prove_contract_call")]
    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: DapenContractFunctionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_ups_cfc_standard_tx")]
    async fn prove_ups_cfc_standard_tx(
        &self,
        input: UPSCFCStandardTransactionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_ups_cfc_deferred_tx")]
    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: UPSCFCDeferredTransactionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_zk_sign")]
    async fn prove_zk_sign(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_zk_sign_inner")]
    async fn prove_zk_sign_inner(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_zk_sign_minifier")]
    async fn prove_zk_sign_minifier(&self, inner_proof: String) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_secp_sign")]
    async fn prove_secp_sign(&self, signature: PsyCompressedSecp256K1Signature) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "register_dpn_software_defined_circuit")]
    async fn register_dpn_software_defined_circuit(
        &self,
        request: QRegisterDPNSoftwareDefinedCircuitRPCRequest,
    ) -> Result<QHashOut<F>, ErrorObjectOwned>;

    #[method(name = "register_plonky2_software_defined_circuit")]
    async fn register_plonky2_software_defined_circuit(
        &self,
        request: QRegisterPlonky2SoftwareDefinedCircuitRPCRequest,
    ) -> Result<QHashOut<F>, ErrorObjectOwned>;

    #[method(name = "prove_software_defined_sign")]
    async fn prove_software_defined_sign(
        &self,
        fingerprint: QHashOut<F>,
        private_key: QHashOut<F>,
        input: DPNSoftwareDefinedSignatureInput,
        sig_hash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    // #[method(name = "finalize_tree")]
    // async fn finalize_tree(&self) -> Result<ProofWithPublicInputs<F, C, D>,
    // ErrorObjectOwned>;

    #[method(name = "prove_ups_end_cap")]
    async fn prove_ups_end_cap(
        &self,
        end_cap_from_proof_tree_input: UPSEndCapFromProofTreeGadgetInput<F>,
        // AggProofRecord
        circuit_type: QStandardBinaryTreeCircuitType,
        fingerprint: QHashOut<F>,
        agg_header: QRecursionAggStandardHeader<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    // #[method(name = "get_verifier_data_by_type")]
    // async fn get_verifier_data_by_type(
    //     &self,
    //     circuit_type: QStandardBinaryTreeCircuitType,
    // ) -> ResultAltVerifierOnlyCircuitData;

    #[method(name = "prove_single_leaf_circuit")]
    async fn prove_single_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<F>,
        single_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        single_proof: ProofWithPublicInputs<F, C, D>,
        single_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_two_leaf_circuit")]
    async fn prove_two_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<F>,
        left_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_two_agg_circuit")]
    async fn prove_two_agg_circuit(
        &self,
        left_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        left_agg_proof_header: QRecursionAggStandardHeader<F>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        right_agg_proof_header: QRecursionAggStandardHeader<F>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_left_leaf_right_agg_circuit")]
    async fn prove_left_leaf_right_agg_circuit(
        &self,
        left_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        right_agg_proof_header: QRecursionAggStandardHeader<F>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_left_agg_right_leaf_circuit")]
    async fn prove_left_agg_right_leaf_circuit(
        &self,
        left_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        left_agg_proof_header: QRecursionAggStandardHeader<F>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalCommonCircuitsData {
    pub ups_start: QCommonCircuitData<F>,
    pub ups_cfc_standard_tx: QCommonCircuitData<F>,
    pub ups_cfc_deferred_tx: QCommonCircuitData<F>,
    pub ups_end_cap: QCommonCircuitData<F>,
    pub zk_circuit: QCommonCircuitData<F>,
    pub secp_circuit: QCommonCircuitData<F>,

    pub ups_circuit_whitelist_root: QHashOut<F>,
    pub ups_start_whitelist_proof: MerkleProofCore<QHashOut<F>>,
    pub ups_cfc_standard_tx_whitelist_proof: MerkleProofCore<QHashOut<F>>,
    pub ups_cfc_deferred_tx_whitelist_proof: MerkleProofCore<QHashOut<F>>,

    // proof_tree_agg_circuits data
    pub single_leaf_circuit: QCommonCircuitData<F>,
    pub two_leaf_circuit: QCommonCircuitData<F>,
    pub two_agg_circuit: QCommonCircuitData<F>,
    pub left_leaf_right_agg_circuit: QCommonCircuitData<F>,
    pub left_agg_right_leaf_circuit: QCommonCircuitData<F>,
    pub leaf_circuit_config_id: u64,
    pub leaf_verifier_data_cap_height: usize,
    pub agg_verifier_data_cap_height: usize,

    pub circuit_inclusion_proofs: SimpleQTreeRecursionManagerInclusionProofs<F>,
}
#[derive(Debug)]
pub struct ProveProxyServerProvider {
    pub rpc_provider: RpcProvider,
    pub contract_circuits: DashMap<u64, Vec<DapenContractFunctionCircuit<C, D>>>,

    pub circuit_manager: Arc<PsyUPSStepCircuitManager<C, D>>,
    pub circuit_info: Arc<SessionCircuitInfoStore<F>>,
    pub circuits_data: LocalCommonCircuitsData,
}

impl ProveProxyServerProvider {
    pub async fn new_with_config(rpc_config: psy_config::NetworkConfigGoldilocks, network_magic: u64) -> anyhow::Result<Self> {
        use psy_common::ups::circuits::LocalCircuitType;
        use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use psy_data::qstore::controllers::session_info::SessionCircuitInfoStore;

        let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;

        let circuit_manager = PsyUPSStepCircuitManager::<C, D>::new_with_config(network_magic);
        let mut circuit_info = SessionCircuitInfoStore::new();

        // circuit_info.register_circuit(
        //     LocalCircuitType::SimpleZKSignature.into(),
        //     zk_circuit.get_fingerprint(),
        //     zk_circuit.get_verifier_config_ref().into(),
        // );
        // circuit_info.register_circuit(
        //     LocalCircuitType::SimpleSecp256K1.into(),
        //     secp_circuit.get_fingerprint(),
        //     secp_circuit.get_verifier_config_ref().into(),
        // );

        circuit_manager.register_info(&mut circuit_info).await;

        let circuits_data = LocalCommonCircuitsData {
            ups_start: QCommonCircuitData {
                fingerprint: circuit_manager.ups_start.get_fingerprint(),
                verifier_config: circuit_manager.ups_start.get_verifier_config_ref().into(),
            },
            ups_cfc_standard_tx: QCommonCircuitData {
                fingerprint: circuit_manager.ups_cfc_standard_tx.get_fingerprint(),
                verifier_config: circuit_manager.ups_cfc_standard_tx.get_verifier_config_ref().into(),
            },
            ups_cfc_deferred_tx: QCommonCircuitData {
                fingerprint: circuit_manager.ups_cfc_deferred_tx.get_fingerprint(),
                verifier_config: circuit_manager.ups_cfc_deferred_tx.get_verifier_config_ref().into(),
            },
            ups_end_cap: QCommonCircuitData {
                fingerprint: circuit_manager.ups_end_cap.get_fingerprint(),
                verifier_config: circuit_manager.ups_end_cap.get_verifier_config_ref().into(),
            },
            ups_circuit_whitelist_root: circuit_manager.ups_circuit_whitelist_root.clone(),
            ups_start_whitelist_proof: circuit_manager.ups_start_whitelist_proof.clone(),
            ups_cfc_standard_tx_whitelist_proof: circuit_manager.ups_cfc_standard_tx_whitelist_proof.clone(),
            ups_cfc_deferred_tx_whitelist_proof: circuit_manager.ups_cfc_deferred_tx_whitelist_proof.clone(),
            single_leaf_circuit: QCommonCircuitData {
                fingerprint: circuit_manager.proof_tree_agg_circuits.circuit_set.single_leaf_circuit.get_fingerprint(),
                verifier_config: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .single_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            two_leaf_circuit: QCommonCircuitData {
                fingerprint: circuit_manager.proof_tree_agg_circuits.circuit_set.two_leaf_circuit.get_fingerprint(),
                verifier_config: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            two_agg_circuit: QCommonCircuitData {
                fingerprint: circuit_manager.proof_tree_agg_circuits.circuit_set.two_agg_circuit.get_fingerprint(),
                verifier_config: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_agg_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            left_leaf_right_agg_circuit: QCommonCircuitData {
                fingerprint: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_leaf_right_agg_circuit
                    .get_fingerprint(),
                verifier_config: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_leaf_right_agg_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            left_agg_right_leaf_circuit: QCommonCircuitData {
                fingerprint: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_agg_right_leaf_circuit
                    .get_fingerprint(),
                verifier_config: circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_agg_right_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            leaf_circuit_config_id: circuit_manager.proof_tree_agg_circuits.circuit_set.leaf_circuit_config_id,
            leaf_verifier_data_cap_height: circuit_manager.proof_tree_agg_circuits.circuit_set.leaf_verifier_data_cap_height,
            agg_verifier_data_cap_height: circuit_manager.proof_tree_agg_circuits.circuit_set.agg_verifier_data_cap_height,
            circuit_inclusion_proofs: circuit_manager.proof_tree_agg_circuits.circuit_inclusion_proofs.clone(),
            zk_circuit: QCommonCircuitData {
                fingerprint: circuit_manager.zk_circuit.get_fingerprint(),
                verifier_config: circuit_manager.zk_circuit.get_verifier_config_ref().into(),
            },
            secp_circuit: QCommonCircuitData {
                fingerprint: circuit_manager.secp_circuit.get_fingerprint(),
                verifier_config: circuit_manager.secp_circuit.get_verifier_config_ref().into(),
            },
        };

        Ok(Self {
            rpc_provider,
            contract_circuits: DashMap::new(),
            circuit_manager: Arc::new(circuit_manager),
            circuit_info: Arc::new(circuit_info),
            circuits_data,
        })
    }

    async fn register_contract_circuits_inner(&self, contract_id: u64) -> anyhow::Result<()> {
        tracing::info!("🔔 register_contract_circuits contract_id: {}", contract_id);
        let mut circuits = Vec::new();
        if self.contract_circuits.get(&contract_id).is_some() {
            tracing::info!("contract {} is already registered", contract_id);
            return Ok(());
        }
        let contract_code = self
            .rpc_provider
            .resolve_get_contract_code(&QSRCmdGetContractCodeDefinition { contract_id })
            .await?;
        for func in contract_code.functions.iter() {
            let dapen_fc = cfc_code_definition_to_dapen_fc(&func)
                .map_err(|err| ErrorObjectOwned::owned(1, "cfc_code_definition_to_dapen_fc error", Some(err.to_string())))?;
            tracing::info!("register contract {} function {}", contract_id, dapen_fc.name);
            circuits.push(DapenContractFunctionCircuit::<C, D>::new(
                &dapen_fc,
                contract_code.state_tree_height as usize,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                false,
            ));
        }
        self.contract_circuits.insert(contract_id, circuits);
        Ok(())
    }
}

#[async_trait]
impl ProveProxyRpcServer for ProveProxyServerProvider {
    async fn prove_ups_start(&self, input: UPSStartStepInput<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_start input");

        let circuit_manager = self.circuit_manager.clone();
        let input = input.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.ups_start.prove_base(&input));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_ups_start: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_ups_start proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn register_contract_circuits(&self, contract_id: u64, contract_code: ContractCodeDefinition) -> Result<(), ErrorObjectOwned> {
        self.register_contract_circuits_inner(contract_id)
            .await
            .map_err(|err| ErrorObjectOwned::owned(1, "register contract circuits error", Some(err.to_string())))
    }

    async fn get_circuits_data(&self) -> Result<String, ErrorObjectOwned> {
        tracing::info!("🔔 get_circuits_data");

        Ok(serde_json::to_string(&self.circuits_data).unwrap())
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> Result<u64, ErrorObjectOwned> {
        tracing::info!("🔔 get_method_id contract_id: {}, method_name: {}", contract_id, method_name);
        if !self.contract_circuits.contains_key(&contract_id) {
            tracing::warn!("contract {} is not registered, can not get method id", contract_id);
            tracing::warn!("register contract {} first", contract_id);
            self.register_contract_circuits_inner(contract_id)
                .await
                .map_err(|err| ErrorObjectOwned::owned(1, "register contract circuits error", Some(err.to_string())))?;
        }
        if let Some(circuits) = self.contract_circuits.get(&contract_id) {
            tracing::info!("get contract {} circuits", contract_id);
            for (id, circuit) in circuits.iter().enumerate() {
                tracing::info!("get contract {} method {} id: {}", contract_id, circuit.fn_def.name, id);
                if circuit.fn_def.name == method_name {
                    tracing::info!("return contract {} method {} id: {}", contract_id, method_name, id);
                    return Ok(id as u64);
                }
            }
        }
        tracing::error!("contract {} method {} not registed", contract_id, method_name);
        Err(ErrorObjectOwned::owned(
            1,
            "get_method_id error",
            Some(format!("contract {} method {} not registed", contract_id, method_name)),
        ))
    }

    async fn get_contract_method_common_data(&self, contract_id: u64, method_id: u32) -> Result<QCommonCircuitData<F>, ErrorObjectOwned> {
        tracing::info!(
            "🔔 get_contract_method_common_data contract_id: {}, method_id: {}",
            contract_id,
            method_id
        );
        if !self.contract_circuits.contains_key(&contract_id) {
            tracing::warn!("contract {} is not registered, can not get method id", contract_id);
            tracing::warn!("register contract {} first", contract_id);
            self.register_contract_circuits_inner(contract_id)
                .await
                .map_err(|err| ErrorObjectOwned::owned(1, "register contract circuits error", Some(err.to_string())))?;
        }

        if let Some(circuits) = self.contract_circuits.get(&contract_id) {
            let circuit = circuits.get(method_id as usize).ok_or_else(|| {
                ErrorObjectOwned::owned(
                    1,
                    format!("contract {} method {} is not found", contract_id, method_id),
                    Some(format!("method_id: {}", method_id)),
                )
            })?;
            tracing::info!(
                "get contract {} method {} common data, fingerprint: {}",
                contract_id,
                method_id,
                circuit.get_fingerprint(),
            );
            return Ok(QCommonCircuitData {
                fingerprint: circuit.get_fingerprint(),
                verifier_config: circuit.get_verifier_config_ref().clone().into(),
            });
        }
        Err(ErrorObjectOwned::owned(
            1,
            format!("contract {} method {} is not found", contract_id, method_id),
            Some(format!("method_id: {}", method_id)),
        ))
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: DapenContractFunctionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_contract_call contract_id: {}, method_id: {}", contract_id, method_id);
        if !self.contract_circuits.contains_key(&contract_id) {
            tracing::warn!("contract {} is not registered, can not get method id", contract_id);
            tracing::warn!("register contract {} first", contract_id);
            self.register_contract_circuits_inner(contract_id)
                .await
                .map_err(|err| ErrorObjectOwned::owned(1, "register contract circuits error", Some(err.to_string())))?;
        }
        if let Some(fn_circuits) = &self.contract_circuits.get(&contract_id) {
            let fn_circuit = fn_circuits.get(method_id as usize).ok_or_else(|| {
                ErrorObjectOwned::owned(
                    1,
                    format!("contract {} method {} is not found", contract_id, method_id),
                    Some(format!("method_id: {}", method_id)),
                )
            })?;

            let input = input.clone();
            let fn_circuit = fn_circuit.clone();

            tokio::task::spawn_blocking(move || {
                fn_circuit
                    .prove_base(&input)
                    .map_err(|err| ErrorObjectOwned::owned(1, "fn_circuit proving error", Some(err.to_string())))
            })
            .await
            .map_err(|join_err| {
                ErrorObjectOwned::owned(
                    1,
                    "prove_software_defined_sign: task schedule failed",
                    Some(format!("Thread pool task execution failed: {}", join_err)),
                )
            })?
        } else {
            Err(ErrorObjectOwned::owned(
                1,
                format!("contract {} method {} is not found", contract_id, method_id),
                Some(format!("method_id: {}", method_id)),
            ))
        }
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: UPSCFCStandardTransactionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_cfc_standard_tx");

        let circuit_manager = self.circuit_manager.clone();
        let input = input.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.ups_cfc_standard_tx.prove_base(&input));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "ups_cfc_standard_tx: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "ups_cfc_standard_tx proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: UPSCFCDeferredTransactionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_cfc_deferred_tx");

        let circuit_manager = self.circuit_manager.clone();
        let input = input.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.ups_cfc_deferred_tx.prove_base(&input));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_ups_cfc_deferred_tx: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_ups_cfc_deferred_tx proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_zk_sign_inner(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_zk_sign_inner");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.zk_circuit.prove_base_inner(private_key, sig_hash));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign_inner: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign_inner proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_zk_sign_minifier(&self, inner_proof: String) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_zk_sign_minifier");

        let circuit_manager = self.circuit_manager.clone();
        let inner_proof = serde_json::from_str::<ProofWithPublicInputs<F, C, D>>(&inner_proof).map_err(|err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign_minifier: inner_proof deserialize error",
                Some(format!("ZK proof deserialize failed: {}", err)),
            )
        })?;

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.zk_circuit.prove_minifier(inner_proof));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign_minifier: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign_minifier proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_zk_sign(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_zk_sign");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.zk_circuit.prove_base(private_key, sig_hash));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_zk_sign proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_secp_sign(&self, signature: PsyCompressedSecp256K1Signature) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_secp_sign");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || circuit_manager.secp_circuit.prove(&signature));

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_secp_sign: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "prove_secp_sign proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn register_dpn_software_defined_circuit(
        &self,
        request: QRegisterDPNSoftwareDefinedCircuitRPCRequest,
    ) -> Result<QHashOut<F>, ErrorObjectOwned> {
        todo!("register_dpn_software_defined_circuit");
    }

    async fn register_plonky2_software_defined_circuit(
        &self,
        request: QRegisterPlonky2SoftwareDefinedCircuitRPCRequest,
    ) -> Result<QHashOut<F>, ErrorObjectOwned> {
        todo!("register_plonky2_software_defined_circuit");
        // let input = SoftwareDefinedSignatureInput::Psy(input);
        // let sdc = SoftwareDefinedSignatureCircuit::new(&input).await;

        // let fingerprint = sdc.get_fingerprint();
        // tracing::info!("register software defined circuit: {}",
        // fingerprint.to_string()); if let Some(_) =
        // self.software_defined_circuits.insert(fingerprint, sdc) {
        //     tracing::warn!("software defined circuit `{}` is already
        // registered", fingerprint.to_string()); };
        // Ok(fingerprint)
    }

    async fn prove_software_defined_sign(
        &self,
        fingerprint: QHashOut<F>,
        private_key: QHashOut<F>,
        input: DPNSoftwareDefinedSignatureInput,
        sig_hash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_software_defined_sign");
        todo!("prove_software_defined_sign");
        // let input = SoftwareDefinedSignatureWitnessInput::Psy(input);

        // if let Some(mut sdc) =
        // self.software_defined_circuits.get_mut(&fingerprint) {
        //     sdc.prove(private_key, &input, sig_hash).await
        //         .map_err(|err| ErrorObjectOwned::owned(1, "software defined
        // signature proving error", Some(err.to_string()))) } else {
        //     Err(ErrorObjectOwned::owned(
        //         1,
        //         format!("software defined circuit {} is not found",
        // fingerprint.to_string()),         Some(format!("fingerprint:
        // {}", fingerprint.to_string())),     ))
        // }
    }

    async fn prove_ups_end_cap(
        &self,
        end_cap_from_proof_tree_input: UPSEndCapFromProofTreeGadgetInput<F>,
        circuit_type: QStandardBinaryTreeCircuitType,
        fingerprint: QHashOut<F>,
        agg_header: QRecursionAggStandardHeader<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_end_cap");

        let circuit_manager = self.circuit_manager.clone();
        let circuit_info = self.circuit_info.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            let agg_whitelist_merkle_proof = circuit_manager
                .proof_tree_agg_circuits
                .circuit_inclusion_proofs
                .get_inclusion_proof_for_type(circuit_type);
            let agg_root_verifier_data = circuit_info
                .get_circuit_info_by_fingerprint(fingerprint)
                .map_err(|err| ErrorObjectOwned::owned(1, "get_circuit_info_by_fingerprint error", Some(err.to_string())))?
                .verifier_data
                .to_verifier_data::<C, D>();

            circuit_manager.ups_end_cap.prove_base(
                &end_cap_from_proof_tree_input,
                &agg_whitelist_merkle_proof,
                &agg_header,
                &proof,
                &agg_root_verifier_data,
            )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "ups_end_cap: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result
            .map_err(|prove_err| ErrorObjectOwned::owned(1, "ups_end_cap proving error", Some(format!("ZK proof generation failed: {}", prove_err))))
    }

    async fn prove_single_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<F>,
        single_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        single_proof: ProofWithPublicInputs<F, C, D>,
        single_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_single_leaf_circuit");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            circuit_manager.proof_tree_agg_circuits.circuit_set.single_leaf_circuit.prove_base(
                agg_circuit_whitelist_root,
                &single_insert_leaf_proof,
                &single_proof,
                &single_verifier_data.to_verifier_data(),
            )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "single_leaf_circuit: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "single_leaf_circuit proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_two_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<F>,
        left_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_two_leaf_circuit");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            circuit_manager.proof_tree_agg_circuits.circuit_set.two_leaf_circuit.prove_base(
                agg_circuit_whitelist_root,
                &left_insert_leaf_proof,
                &left_proof,
                &left_verifier_data.to_verifier_data(),
                &right_insert_leaf_proof,
                &right_proof,
                &right_verifier_data.to_verifier_data(),
            )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "two_leaf_circuit: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "two_leaf_circuit proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_two_agg_circuit(
        &self,
        left_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        left_agg_proof_header: QRecursionAggStandardHeader<F>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        right_agg_proof_header: QRecursionAggStandardHeader<F>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_two_agg_circuit");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            circuit_manager.proof_tree_agg_circuits.circuit_set.two_agg_circuit.prove_base(
                &left_agg_whitelist_merkle_proof,
                &left_agg_proof_header,
                &left_proof,
                &left_verifier_data.to_verifier_data(),
                &right_agg_whitelist_merkle_proof,
                &right_agg_proof_header,
                &right_proof,
                &right_verifier_data.to_verifier_data(),
            )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "two_agg_circuit: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "two_agg_circuit proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_left_leaf_right_agg_circuit(
        &self,
        left_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        right_agg_proof_header: QRecursionAggStandardHeader<F>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_left_leaf_right_agg_circuit");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            circuit_manager
                .proof_tree_agg_circuits
                .circuit_set
                .left_leaf_right_agg_circuit
                .prove_base(
                    &left_insert_leaf_proof,
                    &left_proof,
                    &left_verifier_data.to_verifier_data(),
                    &right_agg_whitelist_merkle_proof,
                    &right_agg_proof_header,
                    &right_proof,
                    &right_verifier_data.to_verifier_data(),
                )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "left_leaf_right_agg_circuit: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "left_leaf_right_agg_circuit proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }

    async fn prove_left_agg_right_leaf_circuit(
        &self,
        left_agg_whitelist_merkle_proof: MerkleProofCore<QHashOut<F>>,
        left_agg_proof_header: QRecursionAggStandardHeader<F>,
        left_proof: ProofWithPublicInputs<F, C, D>,
        left_verifier_data: AltVerifierOnlyCircuitData<F>,
        right_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        right_proof: ProofWithPublicInputs<F, C, D>,
        right_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_left_agg_right_leaf_circuit");

        let circuit_manager = self.circuit_manager.clone();

        let proof_join_handle = tokio::task::spawn_blocking(move || {
            circuit_manager
                .proof_tree_agg_circuits
                .circuit_set
                .left_agg_right_leaf_circuit
                .prove_base(
                    &left_agg_whitelist_merkle_proof,
                    &left_agg_proof_header,
                    &left_proof,
                    &left_verifier_data.to_verifier_data(),
                    &right_insert_leaf_proof,
                    &right_proof,
                    &right_verifier_data.to_verifier_data(),
                )
        });

        let proof_result = proof_join_handle.await.map_err(|join_err| {
            ErrorObjectOwned::owned(
                1,
                "left_agg_right_leaf_circuit: task schedule failed",
                Some(format!("Thread pool task execution failed: {}", join_err)),
            )
        })?;

        proof_result.map_err(|prove_err| {
            ErrorObjectOwned::owned(
                1,
                "left_agg_right_leaf_circuit proving error",
                Some(format!("ZK proof generation failed: {}", prove_err)),
            )
        })
    }
}
