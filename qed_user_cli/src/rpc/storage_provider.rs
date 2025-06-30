use std::fs;
use std::sync::Arc;
use plonky2::field::goldilocks_field::GoldilocksField;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::imm::cmd_processor::{QEDReadCommandBatchOutput, QEDReadCommandProcessorSync};
use crate::rpc::provider::StoreConfig;

#[derive(Debug)]
pub struct StorageProvider {
    pub coordinator_store: Arc<KVQlibmdbxStore>,
    pub realm_store: Arc<KVQlibmdbxStore>,
}

impl StorageProvider {
    pub fn new(config_path: &str) -> anyhow::Result<Self> {
        let config: StoreConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;

        let coordinator_store = Arc::new(KVQlibmdbxStore::new_read(
            &config.coordinator_store_path,
        )?);

        let realm_store = Arc::new(KVQlibmdbxStore::new_read(
            &config.realm_store_path,
        )?);

        anyhow::Ok(Self {
            coordinator_store,
            realm_store,
        })
    }
}

type F = GoldilocksField;

#[maybe_async::maybe_async]
impl QEDReadCommandProcessorSync<F> for StorageProvider {
    async fn resolve_batch(
        &self,
        input: &qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput,
    ) -> anyhow::Result<QEDReadCommandBatchOutput<F>> {
        let mut get_user_leaf = Vec::new();
        for x in &input.get_user_leaf {
            get_user_leaf.push(self.resolve_get_user_leaf(x).await?);
        }
        let mut get_contract_leaf = Vec::new();
        for x in &input.get_contract_leaf {
            get_contract_leaf.push(self.resolve_get_contract_leaf(x).await?);
        }
        let mut get_contract_code = Vec::new();
        for x in &input.get_contract_code {
            get_contract_code.push(self.resolve_get_contract_code(x).await?);
        }
        let mut get_checkpoint_leaf = Vec::new();
        for x in &input.get_checkpoint_leaf {
            get_checkpoint_leaf.push(self.resolve_get_checkpoint_leaf(x).await?);
        }
        let mut get_l2_block_state = Vec::new();
        for x in &input.get_l2_block_state {
            get_l2_block_state.push(self.resolve_get_l2_block_state(x).await?);
        }
        let mut get_merkle_proof = Vec::new();
        for x in &input.get_merkle_proof {
            get_merkle_proof.push(self.resolve_get_merkle_proof(x).await?);
        }
        let mut get_hash = Vec::new();
        for x in &input.get_hash {
            get_hash.push(self.resolve_get_hash(x).await?);
        }
        Ok(QEDReadCommandBatchOutput {
            get_user_leaf: get_user_leaf,
            get_contract_leaf: get_contract_leaf,
            get_contract_code: get_contract_code,
            get_checkpoint_leaf: get_checkpoint_leaf,
            get_l2_block_state: get_l2_block_state,
            get_merkle_proof: get_merkle_proof,
            get_hash: get_hash,
        })
    }

    async fn resolve_get_hash(
        &self,
        input: &qed_store::store::imm::cmd::QSRHashCmd,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        if input.is_realm_cmd() {
            return self.realm_store.resolve_get_hash(input);
        }
        self.coordinator_store.resolve_get_hash(input)
    }

    async fn resolve_get_merkle_proof(
        &self,
        input: &qed_store::store::imm::cmd::QSRMerkleCmd,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        if input.is_realm_cmd() {
            return self.realm_store.resolve_get_merkle_proof(input);
        }
        self.coordinator_store.resolve_get_merkle_proof(input)
    }

    async fn resolve_get_user_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetUserLeafData,
    ) -> anyhow::Result<qed_data::qdata::user::QEDUserLeaf<F>> {
        self.realm_store.resolve_get_user_leaf(input)
    }

    async fn resolve_get_contract_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        self.coordinator_store.resolve_get_contract_leaf(input)
    }

    async fn resolve_get_contract_code(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        self.coordinator_store.resolve_get_contract_code(input)
    }

    async fn resolve_get_checkpoint_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        self.realm_store.resolve_get_checkpoint_leaf(input)
    }

    async fn resolve_get_l2_block_state(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetL2BlockState,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_l2_block_state(input)
    }

    async fn resolve_get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_latest_l2_block_state()
    }
}