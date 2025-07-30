use dashmap::DashMap;
use plonky2::{
    hash::hash_types::{HashOut, RichField},
    plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{
    circuits::traits::qstandard::QStandardCircuit,
    treeprover::qrecursion::standard::manager::{
        leaf_circuit_set::QStandardBinaryRecursionTreeCircuitSet,
        portable::circuits::{
            PortableQTreeRecursionCircuits, PortableQTreeRecursionCircuitsDataTrait,
            PortableQTreeRecursionCircuitsProveTrait, PortableQTreeRecursionCircuitsTrait,
        },
    },
};
use qed_core::{
    config::network_constants::{UPS_CIRCUIT_WHITELIST_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::{alt::AltVerifierOnlyCircuitData, qhashout::QHashOut},
    ups::circuits::LocalCircuitType,
};
use qed_crypto::{
    common::{
        circuit_library::CircuitInfoLibraryBuilder,
        witnesses::qrecursion::proof_data::{
            AggProofRecord, SimpleQTreeRecursionManagerInclusionProofs,
        },
    },
    hash::{
        merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
        traits::hasher::MerkleZeroHasher,
    },
    signature::secp256k1::core::QEDCompressedSecp256K1Signature,
};
use qed_data::{
    qdata::contract::ContractCodeDefinition,
    ups::{
        start_step::UPSStartStepInput,
        ups_cfc_standard_step::{
            UPSCFCDeferredTransactionCircuitInput, UPSCFCStandardTransactionCircuitInput,
        },
        ups_end_cap::UPSEndCapFromProofTreeGadgetInput,
    },
};
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qed_rollup_circuit::ups::circuits::{
    end_cap::UPSStandardEndCapCircuit, ups_cfc_deferred_tx::UPSCFCDeferredTransactionCircuit,
    ups_cfc_standard::UPSCFCStandardTransactionCircuit, ups_start::UPSStartSessionCircuit,
};
use qed_store::controllers::local::session_info::SessionCircuitInfoStore;
use serde::Serialize;
use std::collections::HashMap;

use crate::{
    local::{provider::{
        ProveProxyRpcProvider, ProveProxyRpcTrait}, request::QAggProofRecord,
    },
    dpn::{circuits::cfc::DapenContractFunctionCircuit, data::cfc_code_definition_to_dapen_fc},
};

#[derive(Debug)]
pub struct QEDUPSStepCircuitManager<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub ups_start: UPSStartSessionCircuit<C, D>,
    pub proof_tree_agg_circuits: PortableQTreeRecursionCircuits<C, D>,
    pub ups_cfc_standard_tx: UPSCFCStandardTransactionCircuit<C, D>,
    pub ups_cfc_deferred_tx: UPSCFCDeferredTransactionCircuit<C, D>,
    pub ups_end_cap: UPSStandardEndCapCircuit<C, D>,

    pub ups_circuit_whitelist_root: QHashOut<C::F>,
    pub ups_start_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub ups_cfc_standard_tx_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,
    pub ups_cfc_deferred_tx_whitelist_proof: MerkleProofCore<QHashOut<C::F>>,

    // contract circuits
    pub contract_circuits: DashMap<u64, Vec<DapenContractFunctionCircuit<C, D>>>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> QEDUPSStepCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub fn new_with_config(
        //coset_gate: &GateRef<C::F, D>,
        network_magic: u64,
    ) -> Self {
        let ups_start = UPSStartSessionCircuit::new();
        let ups_cfc_standard_tx = UPSCFCStandardTransactionCircuit::new();
        let ups_cfc_deferred_tx = UPSCFCDeferredTransactionCircuit::new();

        let mut ups_circuit_whitelist_proofs =
            SimpleMerkleTree::<C::Hasher, QHashOut<C::F>>::gen_fast_tree_inclusion_proofs(
                UPS_CIRCUIT_WHITELIST_TREE_HEIGHT,
                &[
                    ups_start.get_fingerprint(),
                    ups_cfc_standard_tx.get_fingerprint(),
                    ups_cfc_deferred_tx.get_fingerprint(),
                ],
            )
            .unwrap();
        let ups_cfc_deferred_tx_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();
        let ups_cfc_standard_tx_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();
        let ups_start_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();

        let ups_circuit_whitelist_root = ups_cfc_standard_tx_whitelist_proof.root;

        let proof_tree_agg_circuits = PortableQTreeRecursionCircuits::new(
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
            1,
            ups_cfc_deferred_tx
                .circuit_data
                .verifier_only
                .constants_sigmas_cap
                .height(),
            &ups_cfc_deferred_tx.circuit_data.common,
        );
        let ups_end_cap = UPSStandardEndCapCircuit::new_with_minifier(
            &proof_tree_agg_circuits
                .circuit_set
                .two_agg_circuit
                .circuit_data
                .common,
            proof_tree_agg_circuits
                .circuit_set
                .two_agg_circuit
                .get_verifier_config_ref()
                .constants_sigmas_cap
                .height(),
            network_magic,
            ups_circuit_whitelist_root,
            proof_tree_agg_circuits
                .circuit_inclusion_proofs
                .circuit_whitelist_tree_root,
        );

        /*
        let ups_end_cap = UPSStandardEndCapCircuit::new_with_minifier(
            proof_tree_agg_circuits.root_circuit.get_common_circuit_data_ref(),
            proof_tree_agg_circuits.root_circuit.get_verifier_config_ref(),
            network_magic,
            ups_circuit_whitelist_root,
        );*/

        Self {
            ups_start,
            proof_tree_agg_circuits,
            ups_cfc_standard_tx,
            ups_cfc_deferred_tx,
            ups_end_cap,
            ups_circuit_whitelist_root,
            ups_start_whitelist_proof,
            ups_cfc_standard_tx_whitelist_proof,
            ups_cfc_deferred_tx_whitelist_proof,
            contract_circuits: DashMap::new(),
        }
    }

    pub fn print_common_config(&self) {
        println!("\n\n\n\n================================\n[ups_start.common]:\n{:?}", self.ups_start.get_common_circuit_data_ref());
        println!("================================\n[ups_cfc_standard_tx.common]:\n{:?}", self.ups_cfc_standard_tx.get_common_circuit_data_ref());
        println!("================================\n[ups_cfc_deferred_tx.common]:\n{:?}", self.ups_cfc_deferred_tx.get_common_circuit_data_ref());
        println!("================================\n[ups_end_cap.common]:\n{:?}", self.ups_end_cap.get_common_circuit_data_ref());

        println!("===============================\n\n\n\n");
        self.proof_tree_agg_circuits.circuit_set.print_common_data();
    }
    /* 
    pub fn register_library<T: CircuitInfoLibraryBuilder<C::F>>(&self, library: &mut T) {

        library.register_circuit(
            LocalCircuitType::UPSStart.into(),
            self.ups_start.get_fingerprint(),
            self.ups_start.get_verifier_config_ref().into()
        );
        library.register_circuit(
            LocalCircuitType::UPSCFCStandard.into(),
            self.ups_cfc_standard_tx.get_fingerprint(),
            self.ups_cfc_standard_tx.get_verifier_config_ref().into()
        );
        library.register_circuit(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.ups_cfc_deferred_tx.get_fingerprint(),
            self.ups_cfc_deferred_tx.get_verifier_config_ref().into()
        );
        library.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into()
        );
        library.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into()
        );


        library.register_whitelist_merkle_proof(
            LocalCircuitType::UPSStart.into(),
            self.ups_start_whitelist_proof.clone(),
        );
        library.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCStandard.into(),
            self.ups_cfc_standard_tx_whitelist_proof.clone(),
        );
        library.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.ups_cfc_deferred_tx_whitelist_proof.clone(),
        );

    }*/
    pub fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        info_store.register_circuit(
            LocalCircuitType::UPSStart.into(),
            self.ups_start.get_fingerprint(),
            self.ups_start.get_verifier_config_ref().into(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCStandard.into(),
            self.ups_cfc_standard_tx.get_fingerprint(),
            self.ups_cfc_standard_tx.get_verifier_config_ref().into(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.ups_cfc_deferred_tx.get_fingerprint(),
            self.ups_cfc_deferred_tx.get_verifier_config_ref().into(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into(),
        );

        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSStart.into(),
            self.ups_start_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCStandard.into(),
            self.ups_cfc_standard_tx_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.ups_cfc_deferred_tx_whitelist_proof.clone(),
        );

        register_qtree_recursion_circuits(&self.proof_tree_agg_circuits.circuit_set, info_store);
        register_qtree_recursion_circuits_whitelist_proofs(
            &self.proof_tree_agg_circuits.circuit_inclusion_proofs,
            info_store,
        );
    }
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D> + 'static, const D: usize> QEDUPSStepCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub async fn prove_ups_start(
        &self,
        input: &UPSStartStepInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.ups_start.prove_base(input)
    }

    pub async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        let mut circuits = Vec::new();
        if self.contract_circuits.get(&contract_id).is_some() {
            tracing::info!("contract {} is already registered", contract_id);
            return Ok(());
        }
        for func in contract_code.functions.iter() {
            let dapen_fc = cfc_code_definition_to_dapen_fc(&func)?;
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

    pub async fn get_method_id(
        &self,
        contract_id: u64,
        method_name: String,
    ) -> anyhow::Result<u64> {
        if let Some(circuits) = self.contract_circuits.get(&contract_id) {
            for (id, circuit) in circuits.iter().enumerate() {
                if circuit.fn_def.name == method_name {
                    return Ok(id as u64);
                }
            }
        }
        Err(anyhow::format_err!(
            "contract {} method {} is not found",
            contract_id,
            method_name
        ))
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        if let Some(circuits) = self.contract_circuits.get(&contract_id) {
            tracing::info!(
                "get contract {} method {} common data",
                contract_id,
                method_id
            );
            let circuit = circuits.get(method_id as usize).ok_or_else(|| {
                anyhow::format_err!("contract {} method {} is not found", contract_id, method_id)
            })?;

            return Ok((
                circuit.get_fingerprint(),
                circuit.get_verifier_config_ref().clone(),
            ));
        }
        Err(anyhow::format_err!(
            "contract {} method {} is not found",
            contract_id,
            method_id
        ))
    }

    pub async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        if let Some(fn_circuits) = &self.contract_circuits.get(&contract_id) {
            let fn_circuit = fn_circuits.get(method_id as usize).ok_or_else(|| {
                anyhow::format_err!("contract {} method {} is not found", contract_id, method_id)
            })?;

            fn_circuit.prove_base(&input)
        } else {
            Err(anyhow::format_err!(
                "contract {} method {} is not found",
                contract_id,
                method_id
            ))
        }
    }

    pub async fn ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.ups_cfc_standard_tx.prove_base(&input)
    }

    pub async fn ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.ups_cfc_deferred_tx.prove_base(&input)
    }

    pub async fn signature(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }

    pub async fn ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &QAggProofRecord<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let agg_whitelist_merkle_proof = self
            .proof_tree_agg_circuits
            .circuit_inclusion_proofs
            .get_inclusion_proof_for_type(agg_proof_record.circuit_type);
        let agg_root_verifier_data = circuit_info
            .get_circuit_info_by_fingerprint(agg_proof_record.fingerprint)?
            .verifier_data
            .to_verifier_data::<C, D>();
        let agg_proof = serde_json::from_str(&agg_proof_record.proof)?;

        self.ups_end_cap.prove_base(
            &end_cap_from_proof_tree_input,
            agg_whitelist_merkle_proof,
            &agg_proof_record.agg_header,
            &agg_proof,
            &agg_root_verifier_data,
        )
    }
}

pub fn register_qtree_recursion_circuits<C: GenericConfig<D>, const D: usize>(
    circuit_set: &QStandardBinaryRecursionTreeCircuitSet<C, D>,
    info_store: &mut SessionCircuitInfoStore<C::F>,
) where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    info_store.register_circuit(
        LocalCircuitType::PTAggSingle.into(),
        circuit_set.single_leaf_circuit.get_fingerprint(),
        circuit_set
            .single_leaf_circuit
            .get_verifier_config_ref()
            .into(),
    );
    info_store.register_circuit(
        LocalCircuitType::PTAggTwoLeaf.into(),
        circuit_set.two_leaf_circuit.get_fingerprint(),
        circuit_set
            .two_leaf_circuit
            .get_verifier_config_ref()
            .into(),
    );
    info_store.register_circuit(
        LocalCircuitType::PTAggTwoAgg.into(),
        circuit_set.two_agg_circuit.get_fingerprint(),
        circuit_set.two_agg_circuit.get_verifier_config_ref().into(),
    );
    info_store.register_circuit(
        LocalCircuitType::PTAggLeftAggRightLeaf.into(),
        circuit_set.left_agg_right_leaf_circuit.get_fingerprint(),
        circuit_set
            .left_agg_right_leaf_circuit
            .get_verifier_config_ref()
            .into(),
    );
    info_store.register_circuit(
        LocalCircuitType::PTAggLeftLeafRightAgg.into(),
        circuit_set.left_leaf_right_agg_circuit.get_fingerprint(),
        circuit_set
            .left_leaf_right_agg_circuit
            .get_verifier_config_ref()
            .into(),
    );
}
pub fn register_qtree_recursion_circuits_whitelist_proofs<F: RichField>(
    inclusion_proofs: &SimpleQTreeRecursionManagerInclusionProofs<F>,
    info_store: &mut SessionCircuitInfoStore<F>,
) {
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggSingle.into(),
        inclusion_proofs.single_leaf_circuit_merkle_proof.clone(),
    );
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggTwoLeaf.into(),
        inclusion_proofs.two_leaf_circuit_merkle_proof.clone(),
    );
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggTwoAgg.into(),
        inclusion_proofs.two_agg_circuit_merkle_proof.clone(),
    );
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggLeftAggRightLeaf.into(),
        inclusion_proofs
            .left_agg_right_leaf_circuit_merkle_proof
            .clone(),
    );
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggLeftLeafRightAgg.into(),
        inclusion_proofs
            .left_leaf_right_agg_circuit_merkle_proof
            .clone(),
    );
}

#[derive(Debug)]
pub enum QCircuitManager<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    Local(QEDUPSStepCircuitManager<C, D>),
    Rpc(ProveProxyRpcProvider<C, D>),
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D> + 'static + Serialize, const D: usize> QCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    pub async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        match self {
            QCircuitManager::Local(manager) => manager.register_info(info_store),
            QCircuitManager::Rpc(provider) => provider.register_info(info_store),
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D> + 'static + Serialize, const D: usize> ProveProxyRpcTrait<C, D>
    for QCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn prove_ups_start(
        &self,
        input: &UPSStartStepInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => manager.prove_ups_start(&input).await,
            QCircuitManager::Rpc(provider) => provider.prove_ups_start(&input).await,
        }
    }

    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        match self {
            QCircuitManager::Local(manager) => {
                manager
                    .register_contract_circuits(contract_id, &contract_code)
                    .await
            }
            QCircuitManager::Rpc(provider) => {
                provider
                    .register_contract_circuits(contract_id, &contract_code)
                    .await
            }
        }
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64> {
        match self {
            QCircuitManager::Local(manager) => {
                manager.get_method_id(contract_id, method_name).await
            }
            QCircuitManager::Rpc(provider) => {
                provider.get_method_id(contract_id, method_name).await
            }
        }
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        match self {
            QCircuitManager::Local(manager) => {
                manager
                    .get_contract_method_common_data(contract_id, method_id)
                    .await
            }
            QCircuitManager::Rpc(provider) => {
                provider
                    .get_contract_method_common_data(contract_id, method_id)
                    .await
            }
        }
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                manager
                    .prove_contract_call(contract_id, method_id, &input)
                    .await
            }
            QCircuitManager::Rpc(provider) => {
                provider
                    .prove_contract_call(contract_id, method_id, &input)
                    .await
            }
        }
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => manager.ups_cfc_standard_tx(&input).await,
            QCircuitManager::Rpc(provider) => provider.prove_ups_cfc_standard_tx(&input).await,
        }
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => manager.ups_cfc_deferred_tx(&input).await,
            QCircuitManager::Rpc(provider) => provider.prove_ups_cfc_deferred_tx(&input).await,
        }
    }

    async fn prove_signature(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }

    async fn prove_secp256k1_signature(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                let agg_whitelist_merkle_proof = manager
                    .proof_tree_agg_circuits
                    .circuit_inclusion_proofs
                    .get_inclusion_proof_for_type(agg_proof_record.circuit_type);
                let agg_root_verifier_data = circuit_info
                    .get_circuit_info_by_fingerprint(agg_proof_record.fingerprint)?
                    .verifier_data
                    .to_verifier_data::<C, D>();
                manager.ups_end_cap.prove_base(
                    &end_cap_from_proof_tree_input,
                    agg_whitelist_merkle_proof,
                    &agg_proof_record.agg_header,
                    &agg_proof_record.proof,
                    &agg_root_verifier_data,
                )
            }
            QCircuitManager::Rpc(provider) => {
                provider
                    .prove_ups_end_cap(
                        &circuit_info,
                        &end_cap_from_proof_tree_input,
                        &agg_proof_record,
                    )
                    .await
            }
        }
    }

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager.ups_start.get_fingerprint()),
            QCircuitManager::Rpc(provider) => provider.ups_start_circuit_fingerprint().await,
        }
    }

    async fn ups_start_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                Ok(manager.ups_start.get_verifier_config_ref().clone().into())
            }
            QCircuitManager::Rpc(provider) => provider.ups_start_circuit_verifier_config().await,
        }
    }

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager.ups_cfc_standard_tx.get_fingerprint()),
            QCircuitManager::Rpc(provider) => {
                provider.ups_cfc_standard_tx_circuit_fingerprint().await
            }
        }
    }

    async fn ups_cfc_standard_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager
                .ups_cfc_standard_tx
                .get_verifier_config_ref()
                .clone()
                .into()),
            QCircuitManager::Rpc(provider) => {
                provider.ups_cfc_standard_tx_circuit_verifier_config().await
            }
        }
    }

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager.ups_cfc_deferred_tx.get_fingerprint()),
            QCircuitManager::Rpc(provider) => {
                provider.ups_cfc_deferred_tx_circuit_fingerprint().await
            }
        }
    }

    async fn ups_cfc_deferred_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager
                .ups_cfc_deferred_tx
                .get_verifier_config_ref()
                .clone()
                .into()),
            QCircuitManager::Rpc(provider) => {
                provider.ups_cfc_deferred_tx_circuit_verifier_config().await
            }
        }
    }

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager.ups_end_cap.get_fingerprint()),
            QCircuitManager::Rpc(provider) => provider.ups_end_cap_circuit_fingerprint().await,
        }
    }

    async fn ups_end_cap_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                Ok(manager.ups_end_cap.get_verifier_config_ref().clone().into())
            }
            QCircuitManager::Rpc(provider) => provider.ups_end_cap_circuit_verifier_config().await,
        }
    }

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>> {
        match self {
            QCircuitManager::Local(manager) => Ok(manager.ups_circuit_whitelist_root),
            QCircuitManager::Rpc(provider) => provider.ups_circuit_whitelist_root().await,
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D>, const D: usize> PortableQTreeRecursionCircuitsDataTrait<C, D>
    for QCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn single_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .single_leaf_circuit_fingerprint().await,
            QCircuitManager::Rpc(provider) => provider.single_leaf_circuit_fingerprint().await,
        }
    }

    async fn two_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .two_leaf_circuit_fingerprint().await,
            QCircuitManager::Rpc(provider) => provider.two_leaf_circuit_fingerprint().await,
        }
    }

    async fn two_agg_circuit_fingerprint(&self) -> QHashOut<C::F> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .two_agg_circuit_fingerprint().await,
            QCircuitManager::Rpc(provider) => provider.two_agg_circuit_fingerprint().await,
        }
    }

    async fn left_leaf_right_agg_circuit_fingerprint(&self) -> QHashOut<C::F> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .left_leaf_right_agg_circuit_fingerprint().await,
            QCircuitManager::Rpc(provider) => provider.left_leaf_right_agg_circuit_fingerprint().await,
        }
    }

    async fn left_agg_right_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .left_agg_right_leaf_circuit_fingerprint().await,
            QCircuitManager::Rpc(provider) => provider.left_agg_right_leaf_circuit_fingerprint().await,
        }
    }

    async fn single_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .single_leaf_circuit_verifier_config().await,
            QCircuitManager::Rpc(provider) => provider.single_leaf_circuit_verifier_config().await,
        }
    }

    async fn two_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .two_leaf_circuit_verifier_config().await,
            QCircuitManager::Rpc(provider) => provider.two_leaf_circuit_verifier_config().await,
        }
    }

    async fn two_agg_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .two_agg_circuit_verifier_config().await,
            QCircuitManager::Rpc(provider) => provider.two_agg_circuit_verifier_config().await,
        }
    }

    async fn left_leaf_right_agg_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .left_leaf_right_agg_circuit_verifier_config().await,
            QCircuitManager::Rpc(provider) => {
                provider.left_leaf_right_agg_circuit_verifier_config().await
            }
        }
    }

    async fn left_agg_right_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .left_agg_right_leaf_circuit_verifier_config().await,
            QCircuitManager::Rpc(provider) => {
                provider.left_agg_right_leaf_circuit_verifier_config().await
            }
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D>, const D: usize> PortableQTreeRecursionCircuitsProveTrait<C, D>
    for QCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn get_verifier_data_by_type(
        &self,
        circuit_type: qed_crypto::common::witnesses::qrecursion::proof_data::QStandardBinaryTreeCircuitType,
    ) -> VerifierOnlyCircuitData<C, D> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .get_verifier_data_by_type(circuit_type).await,
            QCircuitManager::Rpc(provider) => provider.get_verifier_data_by_type(circuit_type).await,
        }
    }

    async fn prove_single_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<C::F>,
        single_insert_leaf_proof: &qed_crypto::hash::merkle::core::DeltaMerkleProofCore<
            QHashOut<C::F>,
        >,
        single_proof: &ProofWithPublicInputs<C::F, C, D>,
        single_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                manager.proof_tree_agg_circuits.prove_single_leaf_circuit(
                    agg_circuit_whitelist_root,
                    single_insert_leaf_proof,
                    single_proof,
                    single_verifier_data,
                ).await
            }
            QCircuitManager::Rpc(provider) => provider.prove_single_leaf_circuit(
                agg_circuit_whitelist_root,
                single_insert_leaf_proof,
                single_proof,
                single_verifier_data,
            ).await,
        }
    }

    async fn prove_two_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<C::F>,
        left_insert_leaf_proof: &qed_crypto::hash::merkle::core::DeltaMerkleProofCore<
            QHashOut<C::F>,
        >,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_insert_leaf_proof: &qed_crypto::hash::merkle::core::DeltaMerkleProofCore<
            QHashOut<C::F>,
        >,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                manager.proof_tree_agg_circuits.prove_two_leaf_circuit(
                    agg_circuit_whitelist_root,
                    left_insert_leaf_proof,
                    left_proof,
                    left_verifier_data,
                    right_insert_leaf_proof,
                    right_proof,
                    right_verifier_data,
                ).await
            }
            QCircuitManager::Rpc(provider) => provider.prove_two_leaf_circuit(
                agg_circuit_whitelist_root,
                left_insert_leaf_proof,
                left_proof,
                left_verifier_data,
                right_insert_leaf_proof,
                right_proof,
                right_verifier_data,
            ).await,
        }
    }

    async  fn prove_two_agg_circuit(
        &self,
        left_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        left_agg_proof_header: &qed_crypto::common::witnesses::qrecursion::header::QRecursionAggStandardHeader<C::F>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        right_agg_proof_header: &qed_crypto::common::witnesses::qrecursion::header::QRecursionAggStandardHeader<C::F>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => {
                manager.proof_tree_agg_circuits.prove_two_agg_circuit(
                    left_agg_whitelist_merkle_proof,
                    left_agg_proof_header,
                    left_proof,
                    left_verifier_data,
                    right_agg_whitelist_merkle_proof,
                    right_agg_proof_header,
                    right_proof,
                    right_verifier_data,
                ).await
            }
            QCircuitManager::Rpc(provider) => provider.prove_two_agg_circuit(
                left_agg_whitelist_merkle_proof,
                left_agg_proof_header,
                left_proof,
                left_verifier_data,
                right_agg_whitelist_merkle_proof,
                right_agg_proof_header,
                right_proof,
                right_verifier_data,
            ).await,
        }
    }

    async  fn prove_left_leaf_right_agg_circuit(
        &self,
        left_insert_leaf_proof: &qed_crypto::hash::merkle::core::DeltaMerkleProofCore<
            QHashOut<C::F>,
        >,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        right_agg_proof_header: &qed_crypto::common::witnesses::qrecursion::header::QRecursionAggStandardHeader<C::F>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .prove_left_leaf_right_agg_circuit(
                    left_insert_leaf_proof,
                    left_proof,
                    left_verifier_data,
                    right_agg_whitelist_merkle_proof,
                    right_agg_proof_header,
                    right_proof,
                    right_verifier_data,
                ).await,
            QCircuitManager::Rpc(provider) => provider.prove_left_leaf_right_agg_circuit(
                left_insert_leaf_proof,
                left_proof,
                left_verifier_data,
                right_agg_whitelist_merkle_proof,
                right_agg_proof_header,
                right_proof,
                right_verifier_data,
            ).await,
        }
    }

    async fn prove_left_agg_right_leaf_circuit(
        &self,
        left_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        left_agg_proof_header: &qed_crypto::common::witnesses::qrecursion::header::QRecursionAggStandardHeader<C::F>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_insert_leaf_proof: &qed_crypto::hash::merkle::core::DeltaMerkleProofCore<
            QHashOut<C::F>,
        >,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        match self {
            QCircuitManager::Local(manager) => manager
                .proof_tree_agg_circuits
                .prove_left_agg_right_leaf_circuit(
                    left_agg_whitelist_merkle_proof,
                    left_agg_proof_header,
                    left_proof,
                    left_verifier_data,
                    right_insert_leaf_proof,
                    right_proof,
                    right_verifier_data,
                ).await,
            QCircuitManager::Rpc(provider) => provider.prove_left_agg_right_leaf_circuit(
                left_agg_whitelist_merkle_proof,
                left_agg_proof_header,
                left_proof,
                left_verifier_data,
                right_insert_leaf_proof,
                right_proof,
                right_verifier_data,
            ).await,
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl<C: GenericConfig<D>, const D: usize> PortableQTreeRecursionCircuitsTrait<C, D>
    for QCircuitManager<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async  fn circuit_inclusion_proofs(&self) -> &SimpleQTreeRecursionManagerInclusionProofs<C::F> {
        match self {
            QCircuitManager::Local(manager) => {
                &manager.proof_tree_agg_circuits.circuit_inclusion_proofs
            }
            QCircuitManager::Rpc(provider) => {
                &provider.common_circuits_data.circuit_inclusion_proofs
            }
        }
    }
}
