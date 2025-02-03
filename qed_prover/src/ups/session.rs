use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::{HashOut, RichField},
    plonk::config::{AlgebraicHasher, GenericConfig},
};
use qed_common_circuit::{circuits::traits::qstandard::QStandardCircuit, treeprover::qrecursion::standard::manager::portable::core::PortableQTreeRecursionManager};
use qed_core::{config::network_constants::{DEFERRED_TRANSACTION_TREE_HEIGHT, INLINE_TRANSACTION_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT}, data::qhashout::QHashOut};
use qed_crypto::{common::witnesses::qrecursion::proof_data::InputLeafProof, hash::traits::hasher::MerkleZeroHasher};
use qed_data::{
    dpn::proving_session::DPNProvingSessionCompactMethodCall, qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf}, ups::{start_step::UPSStartStepInput, ups_context_input::{UserProvingSessionCurrentState, UserProvingSessionHeader}}
};
use qed_store::{
    controllers::local::proving_session::QEDLocalProvingSessionStore,
    store::imm::{cmd::{QSRCmdGetCheckpointLeafData, QSRMerkleCmd, QSRMerkleCmdGetCheckpointTreeMerkleProof, QSRMerkleCmdGetUserTreeMerkleProof}, cmd_processor::{QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}},
};

use super::circuit_manager::core::QEDUPSStepCircuitManager;

const UPS_STEP_LEAF_TYPE: u64 = 1;

#[derive(Clone, Debug)]
pub struct UserProvingSessionManager<
    F: RichField + Extendable<D>,
    H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F>,
    R: QEDReadCommandProcessorSync<F>,
    C: GenericConfig<D, F = F, Hasher = H>,
    const D: usize,
> {
    lps: QEDLocalProvingSessionStore<F, R>,
    proof_tree_state: PortableQTreeRecursionManager<C, D>,
    current_ups_header: UserProvingSessionHeader<F>,
    current_checkpoint_leaf: QEDCheckpointLeaf<F>,
    current_global_state_roots: QEDCheckpointGlobalStateRoots<F>,
    last_ups_step_proof_index: u64,
    
    tx_log: Vec<DPNProvingSessionCompactMethodCall<F>>,
}


type F = GoldilocksField;
const D: usize = 2;
impl<
        H: MerkleZeroHasher<QHashOut<F>> + MerkleZeroHasher<HashOut<F>> + AlgebraicHasher<F>,
        R: QEDReadCommandProcessorSync<F>,
        C: GenericConfig<D, F = F, Hasher = H>,
    > UserProvingSessionManager<F, H, R, C, D>
{
    pub fn new(
        mut lps: QEDLocalProvingSessionStore<F, R>,
        ups_step_circuit_whitelist_root: QHashOut<F>,
    ) -> anyhow::Result<Self> {
        let proof_tree_state = PortableQTreeRecursionManager::<C, D>::new(
            UPS_SESSION_PROOF_TREE_HEIGHT as usize
        );
        let session_start_context = lps.get_ups_start_ctx()?;
        
        let mut new_user=  session_start_context.start_session_user_leaf.clone();

        let latest_checkpoint_id_u64 = lps.get_current_start_checkpoint_id_u64();
        let latest_checkpoint_id_f = lps.get_current_start_checkpoint_id();
        new_user.last_checkpoint_id = latest_checkpoint_id_f;

        let current_checkpoint_leaf = lps
            .cmd_store
            .resolve_get_checkpoint_leaf_mut(&QSRCmdGetCheckpointLeafData { checkpoint_id: latest_checkpoint_id_u64 })?;

        let current_global_state_roots = lps.get_global_state_tree_roots(latest_checkpoint_id_u64)?;




        let current_state = UserProvingSessionCurrentState{
            user_leaf: new_user,
            deferred_tx_debt_tree_root: H::get_zero_hash(DEFERRED_TRANSACTION_TREE_HEIGHT as usize),
            inline_tx_debt_tree_root: H::get_zero_hash(INLINE_TRANSACTION_TREE_HEIGHT as usize),
            tx_hash_stack: QHashOut::ZERO,
            tx_count: F::ZERO,
        };

        let current_ups_header = UserProvingSessionHeader {
            ups_step_circuit_whitelist_root,
            session_start_context,
            current_state,
        };



        Ok(Self {
            lps,
            proof_tree_state,
            current_ups_header,
            current_checkpoint_leaf,
            current_global_state_roots,
            last_ups_step_proof_index: 0,
            tx_log: vec![],
        })
    }

    pub fn get_ups_start_witness(
        &mut self,
    ) -> anyhow::Result<UPSStartStepInput<F>> {
        
        let checkpoint_tree_proof= self.lps.cmd_store.resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetCheckpointTreeMerkleProof(QSRMerkleCmdGetCheckpointTreeMerkleProof{
            checkpoint_id: self.lps.get_current_write_checkpoint_id_u64(),
            leaf_checkpoint_id: self.lps.get_current_start_checkpoint_id_u64(),
        }))?;

        let user_tree_proof =
            self.lps.cmd_store
                .resolve_get_merkle_proof_mut(&QSRMerkleCmd::GetUserTreeMerkleProof(
                    QSRMerkleCmdGetUserTreeMerkleProof {
                        checkpoint_id: self.lps.get_current_write_checkpoint_id_u64(),
                        user_id: self.lps.get_current_user_id_64(),
                    },
                ))?;


        let input = UPSStartStepInput {
            ups_header: self.current_ups_header.clone(),
            checkpoint_leaf: self.current_checkpoint_leaf.clone(),
            state_roots: self.current_global_state_roots.clone(),
            checkpoint_tree_proof,
            user_tree_proof,
        };
        Ok(input)
    }

    pub fn prove_ups_start(&mut self, circuit_mgr: &QEDUPSStepCircuitManager<C, D>) -> anyhow::Result<()> {
        let input = self.get_ups_start_witness()?;
        
        let proof = circuit_mgr.ups_start.prove_base(&input)?;
        self.last_ups_step_proof_index = self.proof_tree_state.injest_single_leaf_proof(InputLeafProof{
            leaf_circuit_type: UPS_STEP_LEAF_TYPE,
            fingerprint: circuit_mgr.ups_start.get_fingerprint(),
            verifier_data: circuit_mgr.ups_start.get_verifier_config_ref().to_owned(),
            proof,
        });


        Ok(())
    }
}
