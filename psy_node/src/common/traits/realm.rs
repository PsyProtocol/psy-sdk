use std::{marker::PhantomData, time::Duration};

use async_trait::async_trait;
use kvq::traits::{KVQBinaryStore, KVQSerializable};
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use psy_common::{
    data::qhashout::QHashOut,
    job::id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID, QProvingJobGraph, QProvingTask},
    utils::graph::BidirectionalGraph,
};
use psy_common_circuit::hash::merkle::gadgets::delta_merkle_proof;
use psy_config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use psy_crypto::hash::{
    merkle::{
        core::{compute_root_merkle_proof_generic, DeltaMerkleProofCore, MerkleProofCore},
        treeprover::{data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition},
        utils::{
            common::{QMerkleNode, SimpleMerkleNodeKey},
            sub_tree_nca::{NCAProofsWithTopLine, UpdateNCAProofsWithDependencies},
        },
    },
    traits::{
        hasher::{FieldQHasher, MerkleHasher},
        qhashable::QFieldHashable,
    },
};
use psy_data::{
    config::store_config::{PsyHash, PsyProof},
    guta::{
        api::SubmitGUTARealmResultAPINoProofInput,
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{VerifyEndCapSimpleStandardInput, VerifySingleEndCapInput, VerifyTwoEndCapCircuitInput},
        stats::GUTAStats,
    },
    models::checkpoint::block_state::BlockStatesModel,
    qdata::{
        checkpoint::{CheckpointSyncInfo, PsyBlockState},
        staging_checkpoint_info::StagingCheckpointInfo,
        ups_end_cap_result::UPSEndCapResultCompact,
        user::PsyUserLeaf,
    },
};
use psy_store::{
    node::realm::PsyRealmStoreReaderAsync,
    queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl, Status},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::realm::state::processor::RealmConfig;

#[async_trait::async_trait]
pub trait CoordinatorClient<F: RichField> {
    async fn get_current_checkpoint_id(&self) -> anyhow::Result<u64>;
    async fn get_latest_block_updates_from_coordinator(
        &self,
        realm_id: u64,
        from_checkpoint: u64,
        to_checkpoint: u64,
    ) -> anyhow::Result<Vec<CheckpointSyncInfo<F>>>;
    async fn get_checkpoint_sync_info(&self, realm_id: u32, checkpoint_id: u64) -> anyhow::Result<CheckpointSyncInfo<F>>;
    async fn submit_guta_v1(&self, input: &SubmitGUTARealmResultAPINoProofInput<F>, proof: &[u8], realm_id: u64) -> anyhow::Result<()>;
    async fn has_pending_guta(&self, realm_id: u32) -> anyhow::Result<bool>;
    async fn get_latest_checkpoint_sync_info(&self, realm_id: u32) -> anyhow::Result<CheckpointSyncInfo<F>>;
}
