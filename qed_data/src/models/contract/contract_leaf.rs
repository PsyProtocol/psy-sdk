use kvq::traits::{
    KVQBinaryStore, KVQStoreAdapter, KVQStoreAdapterReader,
};
use crate::qdata::{checkpoint_id_key::CheckpointTableIdKey, contract::QEDContractLeaf};

use crate::{config::store_config::QEDFelt, models::kvq_merkle::model::CHECKPOINT_ID_FUZZY_SIZE};

pub trait ContractLeafModelReaderCore<
    const CONTRACT_LEAF_TABLE_TYPE: u16,
    S,
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
        .ok_or_else(|| anyhow::anyhow!("Contract not found"))
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

pub trait ContractLeafModelCore<
    const CONTRACT_LEAF_TABLE_TYPE: u16,
    S,
    IDKVA: KVQStoreAdapter<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
>: ContractLeafModelReaderCore<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
    fn set_contract(store: &S, checkpoint_id: u64, contract_id: u64, contract: QEDContractLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,
            contract_id,
        );
        IDKVA::set(store, key_id, contract)?;
        Ok(())
    }
    fn set_contract_ref(store: &S, checkpoint_id: u64, contract_id: u64, contract: &QEDContractLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,
            contract_id,
        );
        IDKVA::set_ref(store, &key_id, contract)?;
        Ok(())
    }
    fn set_contracts(store: &S, checkpoint_id: u64, contract_ids: &[u64], contracts: &[QEDContractLeaf<QEDFelt>]) -> anyhow::Result<()> {
        let key_ids = contract_ids
            .iter()
            .map(|c| {
                CheckpointTableIdKey::<CONTRACT_LEAF_TABLE_TYPE>::new(
                    checkpoint_id,
                    *c
                )
            })
            .collect::<Vec<_>>();
        IDKVA::set_many_split_ref(store, &key_ids, contracts)?;
        Ok(())
    }
}

pub struct ContractLeafModel<const CONTRACT_LEAF_TABLE_TYPE: u16, S, IDKVA> {
    _idkva: IDKVA,
    _store: S,
}

impl<
        const CONTRACT_LEAF_TABLE_TYPE: u16,
        S,
        IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>, QEDContractLeaf<QEDFelt>>,
    > ContractLeafModelReaderCore<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
    for ContractLeafModel<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
}
impl<
        const CONTRACT_LEAF_TABLE_TYPE: u16,
        S,
        IDKVA: KVQStoreAdapter<
            S,
            CheckpointTableIdKey<CONTRACT_LEAF_TABLE_TYPE>,
            QEDContractLeaf<QEDFelt>,
        >,
    > ContractLeafModelCore<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
    for ContractLeafModel<CONTRACT_LEAF_TABLE_TYPE, S, IDKVA>
{
}
