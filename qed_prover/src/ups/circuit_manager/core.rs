use plonky2::{
    hash::hash_types::{HashOut, RichField},
    plonk::config::{AlgebraicHasher, GenericConfig},
};
use qed_common_circuit::{
    circuits::traits::qstandard::QStandardCircuit,
    treeprover::qrecursion::standard::manager::{leaf_circuit_set::QStandardBinaryRecursionTreeCircuitSet, portable::circuits::PortableQTreeRecursionCircuits},
};
use qed_core::{
    config::network_constants::{UPS_CIRCUIT_WHITELIST_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut, ups::circuits::LocalCircuitType,
};
use qed_crypto::{common::{circuit_library::CircuitInfoLibraryBuilder, witnesses::qrecursion::proof_data::SimpleQTreeRecursionManagerInclusionProofs}, hash::{
    merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
    traits::hasher::MerkleZeroHasher,
}};
use qed_rollup_circuit::ups::circuits::{
    end_cap::UPSStandardEndCapCircuit,
    ups_cfc_deferred_tx::UPSCFCDeferredTransactionCircuit,
    ups_cfc_standard::UPSCFCStandardTransactionCircuit,
    ups_start::UPSStartSessionCircuit,
};
use qed_store::controllers::local::session_info::SessionCircuitInfoStore;

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
            &proof_tree_agg_circuits.circuit_set.two_agg_circuit.circuit_data.common,
            proof_tree_agg_circuits.circuit_set.two_agg_circuit.get_verifier_config_ref().constants_sigmas_cap.height(),
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
            self.ups_start.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCStandard.into(),
            self.ups_cfc_standard_tx.get_fingerprint(),
            self.ups_cfc_standard_tx.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.ups_cfc_deferred_tx.get_fingerprint(),
            self.ups_cfc_deferred_tx.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.ups_end_cap.get_fingerprint(),
            self.ups_end_cap.get_verifier_config_ref().into()
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
        register_qtree_recursion_circuits_whitelist_proofs(&self.proof_tree_agg_circuits.circuit_inclusion_proofs, info_store);

        

    }
}



pub fn register_qtree_recursion_circuits<C: GenericConfig<D>, const D: usize>(
    circuit_set: &QStandardBinaryRecursionTreeCircuitSet<C,D>,
    info_store: &mut SessionCircuitInfoStore<C::F>,
) 
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,{

        info_store.register_circuit(
            LocalCircuitType::PTAggSingle.into(),
            circuit_set.single_leaf_circuit.get_fingerprint(),
            circuit_set.single_leaf_circuit.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggTwoLeaf.into(),
            circuit_set.two_leaf_circuit.get_fingerprint(),
            circuit_set.two_leaf_circuit.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggTwoAgg.into(),
            circuit_set.two_agg_circuit.get_fingerprint(),
            circuit_set.two_agg_circuit.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggLeftAggRightLeaf.into(),
            circuit_set.left_agg_right_leaf_circuit.get_fingerprint(),
            circuit_set.left_agg_right_leaf_circuit.get_verifier_config_ref().into()
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggLeftLeafRightAgg.into(),
            circuit_set.left_leaf_right_agg_circuit.get_fingerprint(),
            circuit_set.left_leaf_right_agg_circuit.get_verifier_config_ref().into()
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
        inclusion_proofs.left_agg_right_leaf_circuit_merkle_proof.clone(),
    );
    info_store.register_whitelist_merkle_proof(
        LocalCircuitType::PTAggLeftLeafRightAgg.into(),
        inclusion_proofs.left_leaf_right_agg_circuit_merkle_proof.clone(),
    );
}
