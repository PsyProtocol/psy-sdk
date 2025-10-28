use std::collections::HashMap;

use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
use maybe_async::maybe_async;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::{
        hash_types::RichField,
        hashing::{hash_n_to_hash_no_pad, PlonkyPermutation},
    },
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_core::{
    data::{
        qhashout::QHashOut,
        secp256k1::{bytes_to_u32_vec_le, CompressedPublicKey},
    },
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use psy_crypto::signature::secp256k1::core::QEDCompressedSecp256K1Signature;
use serde::{Deserialize, Serialize};

use crate::local::{
    args::{ContractCallArgs, JobInfo, JobLocation, WorkerJobTracker},
    provider::RpcProvider,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub fn hash_no_pad_compressed_public_key<F: RichField, P: PlonkyPermutation<F>>(secp256k1_public_key: CompressedPublicKey) -> QHashOut<F> {
    let mut secp256k1_public_key_bytes = vec![secp256k1_public_key.0[0], 0, 0, 0];
    secp256k1_public_key_bytes.extend_from_slice(&secp256k1_public_key.0[1..]);
    let secp256k1_public_key_f = bytes_to_u32_vec_le(&secp256k1_public_key_bytes)
        .iter()
        .map(|n| F::from_canonical_u32(*n))
        .collect::<Vec<_>>();

    QHashOut(hash_n_to_hash_no_pad::<F, P>(&secp256k1_public_key_f))
}

pub fn secp256k1_sign<F: RichField>(private_key: SigningKey, sighash: QHashOut<F>) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
    tracing::info!("🔔 prove_secp256k1_signature");

    // let private_key: Hash256 = private_key.into();
    // let private_key = k256::ecdsa::SigningKey::from_slice(&private_key.0)?;
    let public_key = private_key.verifying_key().to_encoded_point(true).to_bytes();
    let mut compressed = [0u8; 33];
    if public_key.len() == 33 {
        compressed.copy_from_slice(&public_key);
    } else {
        return Err(anyhow::format_err!("pub key length is not 33"));
    }
    let pub_compressed = CompressedPublicKey(compressed);
    let result: k256::ecdsa::Signature = private_key.sign_prehash(&sighash.to_le_bytes())?;
    let mut rs_bytes = [0u8; 64];

    let r_bytes = result.r().to_bytes();
    let s_bytes = result.s().to_bytes();
    rs_bytes[0..32].copy_from_slice(&r_bytes);
    rs_bytes[32..64].copy_from_slice(&s_bytes);

    Ok(QEDCompressedSecp256K1Signature {
        public_key: pub_compressed.0,
        signature: rs_bytes,
        message: sighash.into(),
    })
}

pub const MINING_REWARDS_CONTRACT_ID: u64 = 1;
pub const LAST_CLAIMED_CHECKPOINT_SLOT: u64 = 0;

#[derive(Clone)]
pub struct ProofWithCheckpoint {
    pub checkpoint_id: u64,
    pub proof: VariableHeightRewardMerkleProof,
    pub proposed_reward: u64,
}

#[maybe_async]
pub async fn build_claim_calls_for_multi_checkpoints(
    all_proofs: &[ProofWithCheckpoint],
) -> Vec<ContractCallArgs> {
    let mut contract_call_args = Vec::new();

    let total_proofs = all_proofs.len();
    let mut proof_index = 0;

    let count_5s = total_proofs / 5;
    let mut remaining = total_proofs % 5;

    for _ in 0..count_5s {
        let chunk = &all_proofs[proof_index..proof_index + 5];
        let mut batch_inputs = Vec::new();

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.checkpoint_id);
        }

        for proof_with_checkpoint in chunk {
            serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut batch_inputs).await;
        }

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_5".to_string(),
            inputs: batch_inputs,
        });

        proof_index += 5;
    }

    let count_2s = remaining / 2;
    for _ in 0..count_2s {
        let chunk = &all_proofs[proof_index..proof_index + 2];
        let mut batch_inputs = Vec::new();

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.checkpoint_id);
        }

        for proof_with_checkpoint in chunk {
            serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut batch_inputs).await;
        }

        for proof_with_checkpoint in chunk {
            batch_inputs.push(proof_with_checkpoint.proposed_reward);
        }

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_2".to_string(),
            inputs: batch_inputs,
        });

        proof_index += 2;
    }
    remaining = remaining % 2;

    if remaining > 0 {
        let proof_with_checkpoint = &all_proofs[proof_index];
        let mut proof_inputs = Vec::new();

        serialize_proof_to_inputs(&proof_with_checkpoint.proof, &mut proof_inputs).await;

        let mut batch_inputs = vec![proof_with_checkpoint.checkpoint_id];
        batch_inputs.extend(proof_inputs);
        batch_inputs.push(proof_with_checkpoint.proposed_reward);

        contract_call_args.push(ContractCallArgs {
            contract_id: MINING_REWARDS_CONTRACT_ID,
            method_name: "claim_guta_rewards_1".to_string(),
            inputs: batch_inputs,
        });
    }

    contract_call_args
}

#[maybe_async]
pub async fn serialize_proof_to_inputs(proof: &VariableHeightRewardMerkleProof, inputs: &mut Vec<u64>) {
    tracing::debug!("🔍 Serializing proof: {}", serde_json::to_string_pretty(proof).unwrap());

    for j in 0..GUTA_REWARDS_TREE_MAX_HEIGHT {
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
