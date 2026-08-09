use psy_crypto::hash::{
    merkle::core::MerkleProofCore,
    traits::{hasher::PoseidonHasher, qhashable::QFieldHashable},
};
use psy_data::qdata::user::PsyUserLeaf;
use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use psy_common::data::qhashout::QHashOut;

type F = GoldilocksField;

pub(crate) fn is_checkpoint_observable(requested: u64, coordinator: u64, realm: u64) -> bool {
    coordinator >= requested && realm >= requested
}

pub(crate) fn ensure_expected_private_note_root(
    checkpoint: u64,
    observed: QHashOut<F>,
    expected: QHashOut<F>,
) -> Result<(), String> {
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "private note root mismatch at inclusion checkpoint {}: observed={} expected={}",
            checkpoint, observed, expected
        ))
    }
}

pub(crate) fn validate_snapshot_proof(
    label: &str,
    checkpoint: u64,
    proof: &MerkleProofCore<QHashOut<F>>,
    expected_index: u64,
    expected_height: usize,
    expected_root: Option<QHashOut<F>>,
) -> Result<(), String> {
    validate_proof(label, checkpoint, proof, expected_index, expected_height)?;
    if let Some(root) = expected_root {
        if proof.root != root {
            return Err(format!(
                "private note proof invalid at checkpoint {}: {} root={} expected_root={}",
                checkpoint, label, proof.root, root,
            ));
        }
    }
    Ok(())
}

pub(crate) struct PrivateNoteProofCoherence {
    pub checkpoint: u64,
    pub sender_user_id: u64,
    pub contract_id: u64,
    pub note_root_slot: u64,
    pub note_index: u64,
    pub note_tree_height: usize,
    pub contract_state_tree_height: usize,
    pub contract_tree_height: usize,
    pub user_tree_height: usize,
    pub membership_root: QHashOut<F>,
}

pub(crate) fn ensure_private_note_proof_coherence(
    context: &PrivateNoteProofCoherence,
    slot_proof: &MerkleProofCore<QHashOut<F>>,
    contract_proof: &MerkleProofCore<QHashOut<F>>,
    user_leaf: &PsyUserLeaf<F>,
    user_tree_proof: &MerkleProofCore<QHashOut<F>>,
) -> Result<(), String> {
    if context.note_root_slot == 0 {
        return Err("private note proof invalid: note_root_slot must be greater than zero".to_string());
    }
    if !index_fits_height(context.note_index, context.note_tree_height) {
        return Err(format!(
            "private note proof invalid at checkpoint {}: note_index={} exceeds {}-level note tree",
            context.checkpoint, context.note_index, context.note_tree_height,
        ));
    }
    validate_proof(
        "note_root_slot",
        context.checkpoint,
        slot_proof,
        context.note_root_slot,
        context.contract_state_tree_height,
    )?;
    validate_proof(
        "contract",
        context.checkpoint,
        contract_proof,
        context.contract_id,
        context.contract_tree_height,
    )?;
    validate_proof(
        "user_tree",
        context.checkpoint,
        user_tree_proof,
        context.sender_user_id,
        context.user_tree_height,
    )?;
    if context.membership_root != slot_proof.value {
        return Err(format!(
            "private note proof mismatch at checkpoint {}: membership_root={} slot_value={} sender_user_id={} contract_id={} note_root_slot={}",
            context.checkpoint,
            context.membership_root,
            slot_proof.value,
            context.sender_user_id,
            context.contract_id,
            context.note_root_slot,
        ));
    }
    if slot_proof.root != contract_proof.value {
        return Err(format!(
            "private note proof mismatch at checkpoint {}: slot_root={} contract_value={} sender_user_id={} contract_id={}",
            context.checkpoint, slot_proof.root, contract_proof.value, context.sender_user_id, context.contract_id,
        ));
    }
    if contract_proof.root != user_leaf.user_state_tree_root {
        return Err(format!(
            "private note proof mismatch at checkpoint {}: contract_root={} user_state_tree_root={} sender_user_id={}",
            context.checkpoint, contract_proof.root, user_leaf.user_state_tree_root, context.sender_user_id,
        ));
    }
    if user_leaf.user_id.to_canonical_u64() != context.sender_user_id {
        return Err(format!(
            "private note proof mismatch at checkpoint {}: user_leaf_id={} sender_user_id={}",
            context.checkpoint,
            user_leaf.user_id.to_canonical_u64(),
            context.sender_user_id,
        ));
    }
    let user_leaf_hash = user_leaf.qfhash::<PoseidonHasher>();
    if user_leaf_hash != user_tree_proof.value {
        return Err(format!(
            "private note proof mismatch at checkpoint {}: user_leaf_hash={} user_tree_value={} sender_user_id={}",
            context.checkpoint, user_leaf_hash, user_tree_proof.value, context.sender_user_id,
        ));
    }
    Ok(())
}

fn index_fits_height(index: u64, height: usize) -> bool {
    height >= u64::BITS as usize || index < (1u64 << height)
}

fn validate_proof(
    label: &str,
    checkpoint: u64,
    proof: &MerkleProofCore<QHashOut<F>>,
    expected_index: u64,
    expected_height: usize,
) -> Result<(), String> {
    if !index_fits_height(expected_index, expected_height) {
        return Err(format!(
            "private note proof invalid at checkpoint {}: {} index={} exceeds {}-level tree",
            checkpoint, label, expected_index, expected_height,
        ));
    }
    if proof.index != expected_index {
        return Err(format!(
            "private note proof invalid at checkpoint {}: {} index={} expected_index={}",
            checkpoint, label, proof.index, expected_index,
        ));
    }
    if proof.siblings.len() != expected_height {
        return Err(format!(
            "private note proof invalid at checkpoint {}: {} sibling_count={} expected_height={}",
            checkpoint,
            label,
            proof.siblings.len(),
            expected_height,
        ));
    }
    if !proof.verify::<PoseidonHasher>() {
        return Err(format!(
            "private note proof invalid at checkpoint {}: {} advertised_root={} could not be recomputed",
            checkpoint, label, proof.root,
        ));
    }
    Ok(())
}

pub(crate) fn private_note_proof_diagnostic(context: &PrivateNoteProofCoherence) -> String {
    format!(
        "checkpoint={} sender_user_id={} contract_id={} note_root_slot={} note_index={} membership_root={}",
        context.checkpoint,
        context.sender_user_id,
        context.contract_id,
        context.note_root_slot,
        context.note_index,
        context.membership_root,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_expected_private_note_root, ensure_private_note_proof_coherence,
        is_checkpoint_observable, validate_snapshot_proof, PrivateNoteProofCoherence,
    };
    use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
    use psy_common::data::qhashout::QHashOut;
    use psy_crypto::hash::{merkle::core::MerkleProofCore, traits::hasher::PoseidonHasher};
    use psy_data::qdata::user::PsyUserLeaf;

    type F = GoldilocksField;
    // ----------------------------------------------------------------
    // is_checkpoint_observable(requested, coordinator, realm) -> bool
    //
    // Returns true iff both streams have advanced to (>=) the immutable
    // requested checkpoint. Every false branch must be exercised.
    // ----------------------------------------------------------------

    #[test]
    fn observable_returns_false_when_coordinator_ahead_but_realm_lagging() {
        // Coordinator is past the requested checkpoint, but the realm stream
        // has not yet reached it: the checkpoint is NOT observable.
        assert!(!is_checkpoint_observable(100, 150, 80));
    }

    #[test]
    fn observable_returns_false_when_coordinator_lagging_realm_ahead() {
        // Symmetric branch: the coordinator stream has not caught up to the
        // requested checkpoint, so it is not observable even though the realm
        // is ahead.
        assert!(!is_checkpoint_observable(100, 80, 150));
    }

    #[test]
    fn observable_returns_false_when_both_streams_lagging() {
        // Neither stream has reached the requested checkpoint.
        assert!(!is_checkpoint_observable(100, 50, 60));
    }

    #[test]
    fn observable_returns_true_when_both_streams_exactly_at_requested() {
        // Both streams have advanced exactly to the immutable requested
        // checkpoint; it is now observable.
        assert!(is_checkpoint_observable(100, 100, 100));
    }

    #[test]
    fn observable_returns_true_when_both_streams_beyond_requested() {
        // Both streams are past the requested checkpoint.
        assert!(is_checkpoint_observable(100, 200, 180));
    }

    // ----------------------------------------------------------------
    // ensure_expected_private_note_root(checkpoint, observed, expected)
    //   -> Result<(), String>
    //
    // At the transaction inclusion checkpoint, the observed note root must
    // equal the immutable pre-submit expected note root. An exact match
    // succeeds; a mismatch fails with the checkpoint id and BOTH note roots
    // rendered via their real Display form (not source-text).
    // ----------------------------------------------------------------

    #[test]
    fn note_root_exact_match_at_inclusion_checkpoint_succeeds() {
        // The observed note root at the inclusion checkpoint equals the
        // pre-submit expected note root: validation passes.
        let expected = QHashOut::<F>::from_values(1, 2, 3, 4);
        let observed = QHashOut::<F>::from_values(1, 2, 3, 4);
        assert_eq!(
            ensure_expected_private_note_root(142, observed, expected),
            Ok(())
        );
    }

    #[test]
    fn note_root_mismatch_at_inclusion_checkpoint_fails_with_context() {
        // A mismatch must surface the inclusion checkpoint and both note roots
        // (rendered via their real Display form) so callers can tell which
        // checkpoint and which roots diverged. It must NOT silently succeed.
        let expected = QHashOut::<F>::from_values(1, 2, 3, 4);
        let observed = QHashOut::<F>::from_values(9, 8, 7, 6);
        let checkpoint = 142u64;

        let err = ensure_expected_private_note_root(checkpoint, observed, expected)
            .unwrap_err();

        let checkpoint_str = checkpoint.to_string();
        let observed_str = format!("{}", observed);
        let expected_str = format!("{}", expected);

        assert!(
            err.contains(checkpoint_str.as_str()),
            "error must name the inclusion checkpoint, got: {err}"
        );
        assert!(
            err.contains(observed_str.as_str()),
            "error must include the observed note root, got: {err}"
        );
        assert!(
            err.contains(expected_str.as_str()),
            "error must include the expected note root, got: {err}"
        );
    }

    #[test]
    fn note_root_zero_expected_vs_nonzero_observed_fails_with_context() {
        // Boundary: the expected pre-submit root is the zero root but the
        // observed root is non-zero (note not present at this checkpoint).
        // This must fail with checkpoint/root context, not pass silently.
        let expected = QHashOut::<F>::from_values(0, 0, 0, 0);
        let observed = QHashOut::<F>::from_values(0, 0, 0, 1);
        let checkpoint = 7u64;

        let err = ensure_expected_private_note_root(checkpoint, observed, expected)
            .unwrap_err();

        let checkpoint_str = checkpoint.to_string();
        let observed_str = format!("{}", observed);
        let expected_str = format!("{}", expected);

        assert!(
            err.contains(checkpoint_str.as_str()),
            "error must name the inclusion checkpoint, got: {err}"
        );
        assert!(
            err.contains(observed_str.as_str()),
            "error must include the observed note root, got: {err}"
        );
        assert!(
            err.contains(expected_str.as_str()),
            "error must include the expected note root, got: {err}"
        );
    }

    fn proof(index: u64, value: QHashOut<F>, height: usize) -> MerkleProofCore<QHashOut<F>> {
        MerkleProofCore::new_from_params::<PoseidonHasher>(
            index,
            value,
            vec![QHashOut::<F>::from_values(0, 0, 0, 0); height],
        )
    }

    fn user_leaf(sender_user_id: u64, user_state_tree_root: QHashOut<F>) -> PsyUserLeaf<F> {
        PsyUserLeaf {
            public_key: QHashOut::from_values(1, 2, 3, 4),
            user_state_tree_root,
            balance: F::ZERO,
            nonce: F::ZERO,
            last_checkpoint_id: F::from_canonical_u64(142),
            event_index: F::ZERO,
            user_id: F::from_canonical_u64(sender_user_id),
        }
    }

    #[test]
    fn snapshot_proof_rejects_index_outside_tree_height() {
        let out_of_range = proof(1, QHashOut::<F>::from_values(1, 2, 3, 4), 0);
        let err = validate_snapshot_proof(
            "slot",
            142,
            &out_of_range,
            1,
            0,
            None,
        )
        .unwrap_err();
        assert!(err.contains("index=1 exceeds 0-level tree"));
    }

    #[test]
    fn composed_private_note_proofs_accept_one_coherent_snapshot() {
        use psy_crypto::hash::traits::qhashable::QFieldHashable;

        let sender_user_id = 327680;
        let membership_root = QHashOut::<F>::from_values(1, 2, 3, 4);
        let slot_proof = proof(8_388_609, membership_root, 24);
        let contract_proof = proof(0, slot_proof.root, 1);
        let leaf = user_leaf(sender_user_id, contract_proof.root);
        let user_tree_proof = proof(sender_user_id, leaf.qfhash::<PoseidonHasher>(), 20);
        let context = PrivateNoteProofCoherence {
            checkpoint: 142,
            sender_user_id,
            contract_id: 0,
            note_root_slot: 8_388_609,
            note_index: 7,
            note_tree_height: 20,
            contract_state_tree_height: 24,
            contract_tree_height: 1,
            user_tree_height: 20,
            membership_root,
        };

        assert_eq!(
            ensure_private_note_proof_coherence(
                &context,
                &slot_proof,
                &contract_proof,
                &leaf,
                &user_tree_proof,
            ),
            Ok(()),
        );
    }

    #[test]
    fn composed_private_note_proofs_reject_high_note_index_before_proving() {
        use psy_crypto::hash::traits::qhashable::QFieldHashable;

        let sender_user_id = 327680;
        let membership_root = QHashOut::<F>::from_values(1, 2, 3, 4);
        let slot_proof = proof(8_388_609, membership_root, 24);
        let contract_proof = proof(0, slot_proof.root, 1);
        let leaf = user_leaf(sender_user_id, contract_proof.root);
        let user_tree_proof = proof(sender_user_id, leaf.qfhash::<PoseidonHasher>(), 20);
        let context = PrivateNoteProofCoherence {
            checkpoint: 142,
            sender_user_id,
            contract_id: 0,
            note_root_slot: 8_388_609,
            note_index: 7 | (1u64 << 48),
            note_tree_height: 20,
            contract_state_tree_height: 24,
            contract_tree_height: 1,
            user_tree_height: 20,
            membership_root,
        };

        let err = ensure_private_note_proof_coherence(
            &context,
            &slot_proof,
            &contract_proof,
            &leaf,
            &user_tree_proof,
        )
        .unwrap_err();
        assert!(err.contains("note_index"));
        assert!(err.contains(&(7u64 | (1u64 << 48)).to_string()));
    }

    #[test]
    fn composed_private_note_proofs_reject_cross_snapshot_contract_root() {
        use psy_crypto::hash::traits::qhashable::QFieldHashable;

        let sender_user_id = 327680;
        let membership_root = QHashOut::<F>::from_values(1, 2, 3, 4);
        let slot_proof = proof(8_388_609, membership_root, 24);
        let contract_proof = proof(0, slot_proof.root, 1);
        let observed_user_state_root = QHashOut::<F>::from_values(21, 22, 23, 24);
        let leaf = user_leaf(sender_user_id, observed_user_state_root);
        let user_tree_proof = proof(sender_user_id, leaf.qfhash::<PoseidonHasher>(), 20);
        let context = PrivateNoteProofCoherence {
            checkpoint: 142,
            sender_user_id,
            contract_id: 0,
            note_root_slot: 8_388_609,
            note_index: 7,
            note_tree_height: 20,
            contract_state_tree_height: 24,
            contract_tree_height: 1,
            user_tree_height: 20,
            membership_root,
        };

        let err = ensure_private_note_proof_coherence(
            &context,
            &slot_proof,
            &contract_proof,
            &leaf,
            &user_tree_proof,
        )
        .unwrap_err();
        assert!(err.contains(&contract_proof.root.to_string()));
        assert!(err.contains(&observed_user_state_root.to_string()));
    }
}
