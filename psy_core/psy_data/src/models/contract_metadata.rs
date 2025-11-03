use std::marker::PhantomData;

use kvq::traits::{KVQStoreAdapter, KVQStoreAdapterReader};
use plonky2::hash::hash_types::RichField;

use crate::qdata::{
    contract_metadata::ContractMetaData,
    contract_uuid::{ContractTableIdKey, ContractUUID},
};

pub trait ContractMetaDataModelReaderCore<
    const TABLE_TYPE: u16,
    F: RichField,
    S,
    IDKVA: KVQStoreAdapterReader<S, ContractTableIdKey<TABLE_TYPE>, ContractMetaData<F>>,
>
{
    fn get_contract_metadata_by_id(store: &S, contract_uuid: ContractUUID) -> anyhow::Result<ContractMetaData<F>> {
        IDKVA::get_exact(store, &ContractTableIdKey::new(contract_uuid))
            .map_err(|e| anyhow::format_err!("Contract {} Metadata not found, {}", contract_uuid.to_string(), e.to_string()))
    }
    fn get_contract_metadatas_by_id(store: &S, contract_uuids: &[ContractUUID]) -> anyhow::Result<Vec<ContractMetaData<F>>> {
        let keys: Vec<ContractTableIdKey<TABLE_TYPE>> = contract_uuids.iter().map(|id| ContractTableIdKey::new(*id)).collect::<Vec<_>>();
        IDKVA::get_many_exact(store, &keys).map_err(|e| {
            anyhow::format_err!(
                "Contract Metadata {} not found, {}",
                contract_uuids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
                e.to_string()
            )
        })
    }
}

pub trait ContractMetaDataModelCore<
    const TABLE_TYPE: u16,
    F: RichField,
    S,
    IDKVA: KVQStoreAdapter<S, ContractTableIdKey<TABLE_TYPE>, ContractMetaData<F>>,
>: ContractMetaDataModelReaderCore<TABLE_TYPE, F, S, IDKVA>
{
    fn set_contract_metadata(store: &S, contract_uuid: ContractUUID, contract_metadata: ContractMetaData<F>) -> anyhow::Result<()> {
        let key_id = ContractTableIdKey::new(contract_uuid);
        IDKVA::set(store, key_id, contract_metadata)?;
        Ok(())
    }
    fn set_contract_metadatas(store: &S, contract_uuids: &[ContractUUID], contract_metadatas: &[ContractMetaData<F>]) -> anyhow::Result<()> {
        let keys: Vec<ContractTableIdKey<TABLE_TYPE>> = contract_uuids.iter().map(|id| ContractTableIdKey::new(*id)).collect::<Vec<_>>();
        IDKVA::set_many_split_ref(store, &keys, contract_metadatas)?;
        Ok(())
    }
}

pub struct ContractMetaDataModel<const TABLE_TYPE: u16, F: RichField, S, IDKVA> {
    _idkva: IDKVA,
    _store: S,
    _phantom_data: PhantomData<F>,
}

impl<const TABLE_TYPE: u16, F: RichField, S, IDKVA: KVQStoreAdapterReader<S, ContractTableIdKey<TABLE_TYPE>, ContractMetaData<F>>>
    ContractMetaDataModelReaderCore<TABLE_TYPE, F, S, IDKVA> for ContractMetaDataModel<TABLE_TYPE, F, S, IDKVA>
{
}
impl<const TABLE_TYPE: u16, F: RichField, S, IDKVA: KVQStoreAdapter<S, ContractTableIdKey<TABLE_TYPE>, ContractMetaData<F>>>
    ContractMetaDataModelCore<TABLE_TYPE, F, S, IDKVA> for ContractMetaDataModel<TABLE_TYPE, F, S, IDKVA>
{
}
