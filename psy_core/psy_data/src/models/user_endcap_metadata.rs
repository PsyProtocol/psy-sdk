use std::marker::PhantomData;

use kvq::traits::{KVQStoreAdapter, KVQStoreAdapterReader};
use plonky2::hash::hash_types::RichField;

use crate::qdata::{checkpoint_id_key::CheckpointTableIdKey, user_endcap_metadata::UserEndCapMetaData, uuid::UserEndCapUUID};

pub trait UserEndcapMetaDataModelReaderCore<
    const TABLE_TYPE: u16,
    S,
    F: RichField,
    IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<TABLE_TYPE>, UserEndCapMetaData<F>>,
>
{
    fn get_user_endcap_metadata_by_id(store: &S, user_endcap_uuid: UserEndCapUUID) -> anyhow::Result<UserEndCapMetaData<F>> {
        IDKVA::get_exact(store, &user_endcap_uuid.into())
            .map_err(|e| anyhow::format_err!("UserEndcap {} Metadata not found, {}", user_endcap_uuid.to_string(), e.to_string()))
    }
    fn get_user_endcap_metadatas_by_id(store: &S, user_endcap_uuids: &[UserEndCapUUID]) -> anyhow::Result<Vec<UserEndCapMetaData<F>>> {
        let keys: Vec<CheckpointTableIdKey<TABLE_TYPE>> = user_endcap_uuids.iter().map(|id| (*id).into()).collect::<Vec<_>>();
        IDKVA::get_many_exact(store, &keys).map_err(|e| {
            anyhow::format_err!(
                "UserEndcap Metadata {} not found, {}",
                user_endcap_uuids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
                e.to_string()
            )
        })
    }
}

pub trait UserEndcapMetaDataModelCore<
    const TABLE_TYPE: u16,
    S,
    F: RichField,
    IDKVA: KVQStoreAdapter<S, CheckpointTableIdKey<TABLE_TYPE>, UserEndCapMetaData<F>>,
>: UserEndcapMetaDataModelReaderCore<TABLE_TYPE, S, F, IDKVA>
{
    fn set_user_endcap_metadata(store: &S, user_endcap_uuid: UserEndCapUUID, user_endcap_metadata: UserEndCapMetaData<F>) -> anyhow::Result<()> {
        let key_id = user_endcap_uuid.into();
        IDKVA::set(store, key_id, user_endcap_metadata)?;
        Ok(())
    }
    fn set_user_endcap_metadatas(
        store: &S,
        user_endcap_uuids: &[UserEndCapUUID],
        user_endcap_metadatas: &[UserEndCapMetaData<F>],
    ) -> anyhow::Result<()> {
        let keys: Vec<CheckpointTableIdKey<TABLE_TYPE>> = user_endcap_uuids.iter().map(|id| (*id).into()).collect::<Vec<_>>();
        IDKVA::set_many_split_ref(store, &keys, user_endcap_metadatas)?;
        Ok(())
    }
}

pub struct UserEndcapMetaDataModel<const TABLE_TYPE: u16, S, F: RichField, IDKVA> {
    _idkva: IDKVA,
    _store: S,
    _phantom_data: PhantomData<F>,
}

impl<const TABLE_TYPE: u16, F: RichField, S, IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<TABLE_TYPE>, UserEndCapMetaData<F>>>
    UserEndcapMetaDataModelReaderCore<TABLE_TYPE, S, F, IDKVA> for UserEndcapMetaDataModel<TABLE_TYPE, S, F, IDKVA>
{
}
impl<const TABLE_TYPE: u16, F: RichField, S, IDKVA: KVQStoreAdapter<S, CheckpointTableIdKey<TABLE_TYPE>, UserEndCapMetaData<F>>>
    UserEndcapMetaDataModelCore<TABLE_TYPE, S, F, IDKVA> for UserEndcapMetaDataModel<TABLE_TYPE, S, F, IDKVA>
{
}
