use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;
use qed_common_circuit::circuits::zk_signature3::core::QEDBasicZKSignatureCircuit;
use crate::dpn::circuits::cfc::DapenContractFunctionCircuit;
use crate::ups::circuit_manager::core::QEDUPSStepCircuitManager;
use dashmap::DashMap;
use k256::ecdsa::signature::hazmat::PrehashSigner;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_common_circuit::circuits::l1_secp256k1_signature::L1Secp256K1SignatureCircuit;
use qed_core::config::network_constants::UPS_SESSION_PROOF_TREE_HEIGHT;
use qed_core::data::alt::AltVerifierOnlyCircuitData;
use qed_core::data::base_types::hash256::Hash256;
use qed_core::data::qhashout::QHashOut;
use qed_core::data::secp256k1::CompressedPublicKey;
use qed_crypto::common::witnesses::qrecursion::proof_data::QStandardBinaryTreeCircuitType;
use qed_crypto::common::witnesses::qrecursion::proof_data::SimpleQTreeRecursionManagerInclusionProofs;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_crypto::signature;
use qed_crypto::signature::secp256k1;
use qed_crypto::signature::secp256k1::core::QEDCompressedSecp256K1Signature;
use qed_data::qdata::contract::ContractCodeDefinition;

use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_crypto::common::witnesses::qrecursion::header::QRecursionAggStandardHeader;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::ups::start_step::UPSStartStepInput;
use qed_data::ups::ups_cfc_standard_step::UPSCFCDeferredTransactionCircuitInput;
use qed_data::ups::ups_cfc_standard_step::UPSCFCStandardTransactionCircuitInput;

use qed_data::ups::ups_end_cap::UPSEndCapFromProofTreeGadgetInput;
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qed_store::controllers::local::session_info::SessionCircuitInfoStore;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

// use crate::local::provider::LocalCommonCircuitsData;
use crate::local::provider::QCommonCircuitData;
use crate::dpn::data::cfc_code_definition_to_dapen_fc;

type C = PoseidonGoldilocksConfig;
type F = <C as GenericConfig<D>>::F;
const D: usize = 2;

#[rpc(server, client, namespace = "qed")]
pub trait ProveProxyRpc {
    /// local proving proof generate
    #[method(name = "prove_ups_start")]
    async fn prove_ups_start(
        &self,
        input: UPSStartStepInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "get_circuits_data")]
    async fn get_circuits_data(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "get_method_id")]
    async fn get_method_id(
        &self,
        contract_id: u64,
        method_name: String,
    ) -> Result<u64, ErrorObjectOwned>;

    #[method(name = "get_contract_method_common_data")]
    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> Result<QCommonCircuitData<F>, ErrorObjectOwned>;

    #[method(name = "register_contract_circuits")]
    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: ContractCodeDefinition,
    ) -> Result<(), ErrorObjectOwned>;

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

    #[method(name = "prove_signature")]
    async fn prove_signature(
        &self,
        private_key: QHashOut<F>,
        sig_hash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    #[method(name = "prove_secp256k1_signature")]
    async fn prove_secp256k1_signature(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

    // #[method(name = "finalize_tree")]
    // async fn finalize_tree(&self) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned>;

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
    pub contract_circuits: DashMap<u64, Vec<DapenContractFunctionCircuit<C, D>>>,

    pub signature_circuit: QEDBasicZKSignatureCircuit<C, D>,
    pub secp256k1_circuit: L1Secp256K1SignatureCircuit<C, D>,

    pub circuit_manager: QEDUPSStepCircuitManager<C, D>,
    pub circuit_info: SessionCircuitInfoStore<F>,
}

impl ProveProxyServerProvider {
    pub fn new_with_config(network_magic: u64) -> Self {
        use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
        use qed_core::ups::circuits::LocalCircuitType;
        use qed_store::controllers::local::session_info::SessionCircuitInfoStore;

        let signature_circuit = QEDBasicZKSignatureCircuit::<C, D>::new();
        let secp256k1_circuit = L1Secp256K1SignatureCircuit::new();

        let circuit_manager = QEDUPSStepCircuitManager::<C, D>::new_with_config(network_magic);
        let mut circuit_info = SessionCircuitInfoStore::new();

        circuit_info.register_circuit(
            LocalCircuitType::SimpleZKSignature.into(),
            signature_circuit.get_fingerprint(),
            signature_circuit.get_verifier_config_ref().into(),
        );
        circuit_info.register_circuit(
            LocalCircuitType::SimpleSecp256K1.into(),
            secp256k1_circuit.get_fingerprint(),
            secp256k1_circuit.get_verifier_config_ref().into(),
        );

        circuit_manager.register_info(&mut circuit_info);
        Self {
            contract_circuits: DashMap::new(),
            signature_circuit,
            secp256k1_circuit,
            circuit_manager,
            circuit_info,
        }
    }
}

#[async_trait]
impl ProveProxyRpcServer for ProveProxyServerProvider {
    async fn prove_ups_start(
        &self,
        input: UPSStartStepInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_start input");
        self.circuit_manager
            .ups_start
            .prove_base(&input)
            .map_err(|err| {
                ErrorObjectOwned::owned(1, "prove_ups_start proving error", Some(err.to_string()))
            })
    }

    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: ContractCodeDefinition,
    ) -> Result<(), ErrorObjectOwned> {
        tracing::info!("🔔 register_contract_circuits contract_id: {}", contract_id);
        let mut circuits = Vec::new();
        if self.contract_circuits.get(&contract_id).is_some() {
            tracing::info!("contract {} is already registered", contract_id);
            return Ok(());
        }
        for func in contract_code.functions.iter() {
            let dapen_fc = cfc_code_definition_to_dapen_fc(&func).map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "cfc_code_definition_to_dapen_fc error",
                    Some(err.to_string()),
                )
            })?;
            tracing::info!(
                "register contract {} function {}",
                contract_id,
                dapen_fc.name
            );
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

    async fn get_circuits_data(&self) -> Result<String, ErrorObjectOwned> {
        tracing::info!("🔔 get_circuits_data");
        let data = LocalCommonCircuitsData {
            ups_start: QCommonCircuitData {
                fingerprint: self.circuit_manager.ups_start.get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .ups_start
                    .get_verifier_config_ref()
                    .into(),
            },
            ups_cfc_standard_tx: QCommonCircuitData {
                fingerprint: self.circuit_manager.ups_cfc_standard_tx.get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .ups_cfc_standard_tx
                    .get_verifier_config_ref()
                    .into(),
            },
            ups_cfc_deferred_tx: QCommonCircuitData {
                fingerprint: self.circuit_manager.ups_cfc_deferred_tx.get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .ups_cfc_deferred_tx
                    .get_verifier_config_ref()
                    .into(),
            },
            ups_end_cap: QCommonCircuitData {
                fingerprint: self.circuit_manager.ups_end_cap.get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .ups_end_cap
                    .get_verifier_config_ref()
                    .into(),
            },
            ups_circuit_whitelist_root: self.circuit_manager.ups_circuit_whitelist_root.clone(),
            ups_start_whitelist_proof: self.circuit_manager.ups_start_whitelist_proof.clone(),
            ups_cfc_standard_tx_whitelist_proof: self
                .circuit_manager
                .ups_cfc_standard_tx_whitelist_proof
                .clone(),
            ups_cfc_deferred_tx_whitelist_proof: self
                .circuit_manager
                .ups_cfc_deferred_tx_whitelist_proof
                .clone(),
            single_leaf_circuit: QCommonCircuitData {
                fingerprint: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .single_leaf_circuit
                    .get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .single_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            two_leaf_circuit: QCommonCircuitData {
                fingerprint: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_leaf_circuit
                    .get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            two_agg_circuit: QCommonCircuitData {
                fingerprint: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_agg_circuit
                    .get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .two_agg_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            left_leaf_right_agg_circuit: QCommonCircuitData {
                fingerprint: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_leaf_right_agg_circuit
                    .get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_leaf_right_agg_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            left_agg_right_leaf_circuit: QCommonCircuitData {
                fingerprint: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_agg_right_leaf_circuit
                    .get_fingerprint(),
                verifier_config: self
                    .circuit_manager
                    .proof_tree_agg_circuits
                    .circuit_set
                    .left_agg_right_leaf_circuit
                    .get_verifier_config_ref()
                    .into(),
            },
            leaf_circuit_config_id: self
                .circuit_manager
                .proof_tree_agg_circuits
                .circuit_set
                .leaf_circuit_config_id,
            leaf_verifier_data_cap_height: self
                .circuit_manager
                .proof_tree_agg_circuits
                .circuit_set
                .leaf_verifier_data_cap_height,
            agg_verifier_data_cap_height: self
                .circuit_manager
                .proof_tree_agg_circuits
                .circuit_set
                .agg_verifier_data_cap_height,
            circuit_inclusion_proofs: self
                .circuit_manager
                .proof_tree_agg_circuits
                .circuit_inclusion_proofs
                .clone(),
        };

        Ok(serde_json::to_string(&data).unwrap())
    }

    async fn get_method_id(
        &self,
        contract_id: u64,
        method_name: String,
    ) -> Result<u64, ErrorObjectOwned> {
        tracing::info!(
            "🔔 get_method_id contract_id: {}, method_name: {}",
            contract_id,
            method_name
        );
        if let Some(circuits) = self.contract_circuits.get(&contract_id) {
            for (id, circuit) in circuits.iter().enumerate() {
                if circuit.fn_def.name == method_name {
                    return Ok(id as u64);
                }
            }
        }
        Err(ErrorObjectOwned::owned(
            1,
            "get_method_id error",
            Some(format!(
                "contract {} method {} not registed",
                contract_id, method_name
            )),
        ))
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> Result<QCommonCircuitData<F>, ErrorObjectOwned> {
        tracing::info!(
            "🔔 get_contract_method_common_data contract_id: {}, method_id: {}",
            contract_id,
            method_id
        );
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
        tracing::info!(
            "🔔 prove_contract_call contract_id: {}, method_id: {}",
            contract_id,
            method_id
        );
        if let Some(fn_circuits) = &self.contract_circuits.get(&contract_id) {
            let fn_circuit = fn_circuits.get(method_id as usize).ok_or_else(|| {
                ErrorObjectOwned::owned(
                    1,
                    format!("contract {} method {} is not found", contract_id, method_id),
                    Some(format!("method_id: {}", method_id)),
                )
            })?;

            fn_circuit.prove_base(&input).map_err(|err| {
                ErrorObjectOwned::owned(1, "fn_circuit proving error", Some(err.to_string()))
            })
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
        self.circuit_manager
            .ups_cfc_standard_tx
            .prove_base(&input)
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "ups_cfc_standard_tx proving error",
                    Some(err.to_string()),
                )
            })
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: UPSCFCDeferredTransactionCircuitInput<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_ups_cfc_deferred_tx");
        self.circuit_manager
            .ups_cfc_deferred_tx
            .prove_base(&input)
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "ups_cfc_standard_tx proving error",
                    Some(err.to_string()),
                )
            })
    }

    async fn prove_signature(
        &self,
        private_key: QHashOut<F>,
        sig_hash: QHashOut<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_signature");
        self.signature_circuit
            .prove_base(private_key, sig_hash)
            .map_err(|err| {
                ErrorObjectOwned::owned(1, "signature proving error", Some(err.to_string()))
            })
    }

    async fn prove_secp256k1_signature(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_secp256k1_signature");

        self.secp256k1_circuit.prove(&signature).map_err(|err| {
            ErrorObjectOwned::owned(
                1,
                "secp256k1 signature proving error",
                Some(err.to_string()),
            )
        })
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
        let agg_whitelist_merkle_proof = self
            .circuit_manager
            .proof_tree_agg_circuits
            .circuit_inclusion_proofs
            .get_inclusion_proof_for_type(circuit_type);
        let agg_root_verifier_data = self
            .circuit_info
            .get_circuit_info_by_fingerprint(fingerprint)
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "get_circuit_info_by_fingerprint error",
                    Some(err.to_string()),
                )
            })?
            .verifier_data
            .to_verifier_data::<C, D>();

        self.circuit_manager
            .ups_end_cap
            .prove_base(
                &end_cap_from_proof_tree_input,
                &agg_whitelist_merkle_proof,
                &agg_header,
                &proof,
                &agg_root_verifier_data,
            )
            .map_err(|err| {
                ErrorObjectOwned::owned(1, "ups_end_cap proving error", Some(err.to_string()))
            })
    }

    async fn prove_single_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<F>,
        single_insert_leaf_proof: DeltaMerkleProofCore<QHashOut<F>>,
        single_proof: ProofWithPublicInputs<F, C, D>,
        single_verifier_data: AltVerifierOnlyCircuitData<F>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ErrorObjectOwned> {
        tracing::info!("🔔 prove_single_leaf_circuit");
        self.circuit_manager
            .proof_tree_agg_circuits
            .circuit_set
            .single_leaf_circuit
            .prove_base(
                agg_circuit_whitelist_root,
                &single_insert_leaf_proof,
                &single_proof,
                &single_verifier_data.to_verifier_data(),
            )
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "single_leaf_circuit proving error",
                    Some(err.to_string()),
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
        self.circuit_manager
            .proof_tree_agg_circuits
            .circuit_set
            .two_leaf_circuit
            .prove_base(
                agg_circuit_whitelist_root,
                &left_insert_leaf_proof,
                &left_proof,
                &left_verifier_data.to_verifier_data(),
                &right_insert_leaf_proof,
                &right_proof,
                &right_verifier_data.to_verifier_data(),
            )
            .map_err(|err| {
                ErrorObjectOwned::owned(1, "two_leaf_circuit proving error", Some(err.to_string()))
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
        self.circuit_manager
            .proof_tree_agg_circuits
            .circuit_set
            .two_agg_circuit
            .prove_base(
                &left_agg_whitelist_merkle_proof,
                &left_agg_proof_header,
                &left_proof,
                &left_verifier_data.to_verifier_data(),
                &right_agg_whitelist_merkle_proof,
                &right_agg_proof_header,
                &right_proof,
                &right_verifier_data.to_verifier_data(),
            )
            .map_err(|err| {
                ErrorObjectOwned::owned(1, "two_agg_circuit proving error", Some(err.to_string()))
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
        self.circuit_manager
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
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "left_leaf_right_agg_circuit proving error",
                    Some(err.to_string()),
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
        self.circuit_manager
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
            .map_err(|err| {
                ErrorObjectOwned::owned(
                    1,
                    "left_agg_right_leaf_circuit proving error",
                    Some(err.to_string()),
                )
            })
    }
}
