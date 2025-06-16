use std::fs;
use plonky2::field::goldilocks_field::GoldilocksField;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::store::imm::cmd_processor::QEDReadCommandProcessorSync;
use crate::rpc::provider::StoreConfig;

#[derive(Debug)]
pub struct StorageProvider {
    pub coordinator_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
    pub realm_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
}

impl StorageProvider {
    pub fn new(config_path: &str) -> anyhow::Result<Self> {
        let config: StoreConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;

        let coordinator_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(
                &config.coordinator_store_path,
            )?);

        let realm_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(
                &config.realm_store_path,
            )?);

        anyhow::Ok(Self {
            coordinator_store,
            realm_store,
        })
    }
}

type F = GoldilocksField;
impl QEDReadCommandProcessorSync<F> for StorageProvider {
    fn resolve_batch(
        &self,
        input: &qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput,
    ) -> anyhow::Result<qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput<F>> {
        anyhow::Ok(
            qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput::<F> {
                get_user_leaf: input
                    .get_user_leaf
                    .iter()
                    .map(|x| self.resolve_get_user_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_contract_leaf: input
                    .get_contract_leaf
                    .iter()
                    .map(|x| self.resolve_get_contract_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_contract_code: input
                    .get_contract_code
                    .iter()
                    .map(|x| self.resolve_get_contract_code(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_checkpoint_leaf: input
                    .get_checkpoint_leaf
                    .iter()
                    .map(|x| self.resolve_get_checkpoint_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_l2_block_state: input
                    .get_l2_block_state
                    .iter()
                    .map(|x| self.resolve_get_l2_block_state(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_merkle_proof: input
                    .get_merkle_proof
                    .iter()
                    .map(|x| self.resolve_get_merkle_proof(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_hash: input
                    .get_hash
                    .iter()
                    .map(|x| self.resolve_get_hash(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
            },
        )
    }

    fn resolve_get_hash(
        &self,
        input: &qed_store::store::imm::cmd::QSRHashCmd,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        if input.is_realm_cmd() {
            return self.realm_store.resolve_get_hash(input);
        }
        self.coordinator_store.resolve_get_hash(input)
    }

    fn resolve_get_merkle_proof(
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

    fn resolve_get_user_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetUserLeafData,
    ) -> anyhow::Result<qed_data::qdata::user::QEDUserLeaf<F>> {
        self.realm_store.resolve_get_user_leaf(input)
    }

    fn resolve_get_contract_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        self.coordinator_store.resolve_get_contract_leaf(input)
    }

    fn resolve_get_contract_code(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        self.coordinator_store.resolve_get_contract_code(input)
    }

    fn resolve_get_checkpoint_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        self.realm_store.resolve_get_checkpoint_leaf(input)
    }

    fn resolve_get_l2_block_state(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetL2BlockState,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_l2_block_state(input)
    }

    fn resolve_get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_latest_l2_block_state()
    }
}