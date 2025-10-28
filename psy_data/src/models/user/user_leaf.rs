use kvq::traits::{
    KVQBinaryStore, KVQStoreAdapter, KVQStoreAdapterReader,
};
use plonky2::field::types::PrimeField64;
use crate::qdata::{checkpoint_id_key::CheckpointTableIdKey, user::QEDUserLeaf};

use crate::{config::store_config::QEDFelt, models::kvq_merkle::model::CHECKPOINT_ID_FUZZY_SIZE};

pub trait UserLeafModelReaderCore<
    const USER_LEAF_TABLE_TYPE: u16,
    S,
    IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>, QEDUserLeaf<QEDFelt>>,
>
{
    fn get_user_by_id(
        store: &S,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<QEDFelt>> {
        IDKVA::get_leq(
            store,
            &CheckpointTableIdKey::new(checkpoint_id, user_id),
            CHECKPOINT_ID_FUZZY_SIZE,
        )?
        .ok_or_else(|| anyhow::anyhow!("User not found"))
    }
    fn get_users_by_id(
        store: &S,
        checkpoint_id: u64,
        user_ids: &[u64],
    ) -> anyhow::Result<Vec<QEDUserLeaf<QEDFelt>>> {
        let keys = user_ids
            .iter()
            .map(|id| CheckpointTableIdKey::new(checkpoint_id, *id))
            .collect::<Vec<_>>();
        IDKVA::get_many_leq_u(store, &keys, CHECKPOINT_ID_FUZZY_SIZE)
    }
}

pub trait UserLeafModelCore<
    const USER_LEAF_TABLE_TYPE: u16,
    S,
    IDKVA: KVQStoreAdapter<S, CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>, QEDUserLeaf<QEDFelt>>,
>: UserLeafModelReaderCore<USER_LEAF_TABLE_TYPE, S, IDKVA>
{
    fn set_user(store: &S, checkpoint_id: u64, user: QEDUserLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,//user.last_checkpoint_id.to_canonical_u64(),
            user.user_id.to_canonical_u64(),
        );
        IDKVA::set(store, key_id, user)?;
        Ok(())
    }
    fn set_user_ref(store: &S, checkpoint_id: u64, user: &QEDUserLeaf<QEDFelt>) -> anyhow::Result<()> {
        let key_id = CheckpointTableIdKey::new(
            checkpoint_id,//user.last_checkpoint_id.to_canonical_u64(),
            user.user_id.to_canonical_u64(),
        );
        IDKVA::set_ref(store, &key_id, user)?;
        Ok(())
    }
    fn set_users(store: &S, checkpoint_id: u64, users: &[QEDUserLeaf<QEDFelt>]) -> anyhow::Result<()> {
        let key_ids = users
            .iter()
            .map(|u| {
                CheckpointTableIdKey::<USER_LEAF_TABLE_TYPE>::new(
                    //u.last_checkpoint_id.to_canonical_u64(),
                    checkpoint_id,
                    u.user_id.to_canonical_u64(),
                )
            })
            .collect::<Vec<_>>();
        IDKVA::set_many_split_ref(store, &key_ids, users)?;
        Ok(())
    }
}

pub struct UserLeafModel<const USER_LEAF_TABLE_TYPE: u16, S, IDKVA> {
    _idkva: IDKVA,
    _store: S,
}

impl<
        const USER_LEAF_TABLE_TYPE: u16,
        S,
        IDKVA: KVQStoreAdapterReader<S, CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>, QEDUserLeaf<QEDFelt>>,
    > UserLeafModelReaderCore<USER_LEAF_TABLE_TYPE, S, IDKVA>
    for UserLeafModel<USER_LEAF_TABLE_TYPE, S, IDKVA>
{
}
impl<
        const USER_LEAF_TABLE_TYPE: u16,
        S,
        IDKVA: KVQStoreAdapter<
            S,
            CheckpointTableIdKey<USER_LEAF_TABLE_TYPE>,
            QEDUserLeaf<QEDFelt>,
        >,
    > UserLeafModelCore<USER_LEAF_TABLE_TYPE, S, IDKVA>
    for UserLeafModel<USER_LEAF_TABLE_TYPE, S, IDKVA>
{
}
