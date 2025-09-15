use plonky2::{
    field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::target::Target, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}
};
use qed_common_circuit::builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore, core::CircuitBuilderHelpersCore};
use qed_core::config::network_constants::{CHECKPOINT_TREE_HEIGHT, GUTA_FEE, TOKEN_CONTRACT_ID, TOKEN_SIMPLE_BURN_METHOD_ID};

use crate::{gadgets::qdata::{
    ups_context_input::UserProvingSessionHeaderGadget, user_contract_state::{SignContextGadget, UserContractStateGadget}, contract_function_call::DPNProvingSessionSimpleMethodCallGadget}, guta::gadgets::guta_stats::GUTAStatsGadget}
;

use super::{ups_end_cap_result::UPSEndCapResultCompactGadget, ups_signature_data::QEDUserProvingSessionSignatureDataCompactGadget};


#[derive(Clone, Debug)]
pub struct UPSEndCapCoreGadget {
    // start require witness

    // start computed
    pub sig_data_compact_gadget: QEDUserProvingSessionSignatureDataCompactGadget,
    pub end_cap_result_gadget: UPSEndCapResultCompactGadget,
    pub guta_stats: GUTAStatsGadget,
}
const MAX_NONCE_BITS: u8 = 32;

impl UPSEndCapCoreGadget {
    pub fn enforce_signature_constraints<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        last_header_gadget: &UserProvingSessionHeaderGadget,
        sig_proof_public_inputs_hash: HashOutTarget,
        sig_proof_fingerprint: HashOutTarget,
        sig_proof_param_hash: HashOutTarget,
        nonce: Target,
        slots_modified: Target,
        network_magic: u64,
        empty_deferred_tx_debt_tree_root: HashOutTarget,
        empty_inline_tx_debt_tree_root: HashOutTarget,
        second_to_last_tx_hash_stack: HashOutTarget,
    ) -> Self {
        builder.connect(
            last_header_gadget.current_state.user_leaf.nonce,
            last_header_gadget.session_start_context.start_session_user_leaf.nonce,
        );

        builder.ensure_is_greater_than(
            MAX_NONCE_BITS as usize,
            nonce,
            last_header_gadget.current_state.user_leaf.nonce,
        );

        let mut real_end_user_leaf = last_header_gadget.current_state.user_leaf.clone();
        real_end_user_leaf.nonce = nonce;

        let end_user_leaf_hash = real_end_user_leaf.to_hash::<H,F,D>(builder);

        let expected_public_key = builder.hash_two_to_one::<H>(
            sig_proof_fingerprint,
            sig_proof_param_hash,
        );

        builder.connect_hashes(
            last_header_gadget.session_start_context.start_session_user_leaf.public_key,
            real_end_user_leaf.public_key,
        );
        builder.connect_hashes(
            last_header_gadget.session_start_context.start_session_user_leaf.public_key,
            expected_public_key,
        );



        builder.connect(
            real_end_user_leaf.user_id,
            last_header_gadget.session_start_context.start_session_user_leaf.user_id,
        );


        let start_user_leaf_hash = last_header_gadget.session_start_context.start_session_user_leaf_hash;

        let sig_data_compact_gadget = QEDUserProvingSessionSignatureDataCompactGadget {
            start_user_leaf_hash,
            end_user_leaf_hash,
            checkpoint_leaf_hash: last_header_gadget.session_start_context.checkpoint_leaf_hash,
            tx_stack_hash: last_header_gadget.current_state.tx_hash_stack,
            tx_count: last_header_gadget.current_state.tx_count,
        };

        let sign_context = SignContextGadget {
            checkpoint_tree_root: last_header_gadget
                .session_start_context
                .checkpoint_tree_root,
            user_leaf: last_header_gadget.current_state.user_leaf,
        };

        let ups_end_cap_sighash = sig_data_compact_gadget.get_sig_action_with_user_info::<H,F,D>(
            builder,
            network_magic,
            last_header_gadget.session_start_context.start_session_user_leaf.user_id,
            nonce,
            &sign_context,
        ).sig_action_hash;

        let expected_public_inputs_hash = builder.hash_two_to_one::<H>(
            ups_end_cap_sighash,
            sig_proof_param_hash,
        );

        builder.connect_hashes(
            expected_public_inputs_hash,
            sig_proof_public_inputs_hash,
        );


        let end_cap_result_gadget = UPSEndCapResultCompactGadget{
            start_user_leaf_hash,
            end_user_leaf_hash,
            checkpoint_tree_root_hash: last_header_gadget.session_start_context.checkpoint_tree_root,
            user_id: last_header_gadget.session_start_context.start_session_user_leaf.user_id,
        };



        builder.connect(
            last_header_gadget.session_start_context.checkpoint_id,
            real_end_user_leaf.last_checkpoint_id,
        );

        builder.ensure_is_greater_than(
            CHECKPOINT_TREE_HEIGHT as usize,
            real_end_user_leaf.last_checkpoint_id,
            last_header_gadget.session_start_context.start_session_user_leaf.last_checkpoint_id,
        );




        // ensure the deferred tx debt tree is empty
        builder.connect_hashes(
            last_header_gadget.current_state.deferred_tx_debt_tree_root,
            empty_deferred_tx_debt_tree_root,
        );

        // ensure the inline tx debt tree is empty
        builder.connect_hashes(
            last_header_gadget.current_state.inline_tx_debt_tree_root,
            empty_inline_tx_debt_tree_root,
        );


        let one_target = builder.one();

        let tx_count = last_header_gadget.current_state.tx_count;
        builder.ensure_is_greater_than(MAX_NONCE_BITS as usize, tx_count, one_target);

        let burn_contract_id = builder.constant_u64(TOKEN_CONTRACT_ID as u64);
        let burn_method_id = builder.constant_u64(TOKEN_SIMPLE_BURN_METHOD_ID as u64);
        let burn_amount = builder.constant_u64(GUTA_FEE);

        let expected_burn_transaction = DPNProvingSessionSimpleMethodCallGadget {
            contract_id: burn_contract_id,
            method_id: burn_method_id,
            inputs: vec![burn_amount],
        };

        let current_tx_stack = last_header_gadget.current_state.tx_hash_stack;
        let expected_burn_tx_hash = expected_burn_transaction.to_hash::<H,F,D>(builder);

        let reconstructed_current_stack = builder.hash_two_to_one::<H>(
            second_to_last_tx_hash_stack,
            expected_burn_tx_hash
        );
        builder.connect_hashes(reconstructed_current_stack, current_tx_stack);

        let guta_stats = GUTAStatsGadget{
            fees_collected: burn_amount,
            user_ops_processed: one_target,
            total_transactions: tx_count,
            slots_modified: slots_modified,
        };

        Self {
            sig_data_compact_gadget,
            end_cap_result_gadget,
            guta_stats,
        }
    }
}

