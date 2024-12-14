use kvq::traits::{
    KVQBinaryStoreImmutable, KVQBinaryStoreReader, KVQStoreAdapterImmutable, KVQStoreAdapterReader,
};
use qed_data::qdata::{checkpoint_id_key::CheckpointTableIdKey, contract::QEDContractLeaf};

use crate::{config::store_config::QEDFelt, models::kvq_merkle::model::CHECKPOINT_ID_FUZZY_SIZE};

pub trait ContractLeafModelReaderCore<
    const CONTRACT_LEAF_TABLE_TYPE: u16,
    S: KVQBinaryStoreReader,
    IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
>
{
    fn get_contract_by_id(
        store: &S,
        checkpoint_id: u64,
        contract_id: u64,
    ) -> anyhow::Result<QEDContractLeaf<QEDFelt>> {
        IDKVA::get_leq(
            store,
            &CheckpointTableIdKey::new(checkpoint_id, contract_id),
            CHECKPOINT_ID_FUZZY_SIZE,
        )?
        .ok_or_else(|| anyhow::anyhow!("User not found"))
    }
    fn get_contracts_by_id(
        store: &S,
        checkpoint_id: u64,
        contract_ids: &[u64],
    ) -> anyhow::Result<Vec<QEDContractLeaf<QEDFelt>>> {
        let keys = contract_ids
            .iter()
            .map(|id| CheckpointTableIdKey::new(checkpoint_id, *id))
            .collect::<Vec<_>>();
        IDKVA::get_many_leq_u(store, &keys, CHECKPOINT_ID_FUZZY_SIZE)
    }
}

pub trait ContractLeafModelCoreImmutable<
    const CONTRACT_LEAF_TABLE_TYPE: u16,
    S: KVQBinaryStoreImmutable,
    IDKVA: KVQStoreAdapterImmutable<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
>: ContractLeafModelReaderCore<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
    fn set_contract_imm(store: &S, checkpoint_id: u64, contract_id: u64, contract: QEDContractLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,
            contract_id,
        );
        IDKVA::imm_set(store, key_id, contract)?;
        Ok(())
    }
    fn set_contract_ref_imm(store: &S, checkpoint_id: u64, contract_id: u64, contract: &QEDContractLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,
            contract_id,
        );
        IDKVA::imm_set_ref(store, &key_id, contract)?;
        Ok(())
    }
    fn set_contracts_imm(store: &S, checkpoint_id: u64, contract_ids: &[u64], contracts: &[QEDContractLeaf<QEDFelt>]) -> anyhow::Result<()> {
        let key_ids = contract_ids
            .iter()
            .map(|c| {
                CheckpointTableIdKey::<CONTRACT_LEAF_TABLE_TYPE>::new(
                    checkpoint_id,
                    *c
                )
            })
            .collect::<Vec<_>>();
        IDKVA::imm_set_many_split_ref(store, &key_ids, contracts)?;
        Ok(())
    }
}

pub struct ContractLeafModel<const CONTRACT_LEAF_TABLE_TYPE: u16, S, IDKVA> {
    _idkva: IDKVA,
    _store: S,
}

impl<
        const CONTRACT_LEAF_TABLE_TYPE: u16,
        S: KVQBinaryStoreReader,
        IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
    > ContractLeafModelReaderCore<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
    for ContractLeafModel<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
}
impl<
        const CONTRACT_LEAF_TABLE_TYPE: u16,
        S: KVQBinaryStoreImmutable,
        IDKVA: KVQStoreAdapterImmutable<
            S,
            CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>,
            QEDContractLeaf<QEDFelt>,
        >,
    > ContractLeafModelCoreImmutable<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
    for ContractLeafModel<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
}
