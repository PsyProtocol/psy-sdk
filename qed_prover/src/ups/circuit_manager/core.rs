use plonky2::{
    hash::hash_types::HashOut,
    plonk::config::{AlgebraicHasher, GenericConfig},
};
use qed_common_circuit::{
    circuits::traits::qstandard::QStandardCircuit,
    treeprover::qrecursion::standard::manager::portable::circuits::PortableQTreeRecursionCircuits,
};
use qed_core::{
    config::network_constants::{UPS_CIRCUIT_WHITELIST_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use qed_crypto::hash::{
    merkle::{core::MerkleProofCore, utils::simple_merkle_tree::SimpleMerkleTree},
    traits::hasher::MerkleZeroHasher,
};
use qed_rollup_circuit::ups::circuits::{
    end_cap::UPSStandardEndCapCircuit,
    ups_cfc_deferred_tx::UPSCFCDeferredTransactionCircuit,
    ups_cfc_standard::UPSCFCStandardTransactionCircuit,
    ups_start::UPSStartSessionCircuit,
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

        let ups_end_cap = UPSStandardEndCapCircuit::new(
            &ups_cfc_deferred_tx.circuit_data.common,
            ups_cfc_deferred_tx
                .circuit_data
                .verifier_only
                .constants_sigmas_cap
                .height(),
            network_magic,
            ups_circuit_whitelist_root,
            proof_tree_agg_circuits
                .circuit_inclusion_proofs
                .circuit_whitelist_tree_root,
        );

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
}
