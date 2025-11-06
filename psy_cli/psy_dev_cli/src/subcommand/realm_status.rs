use core::time;
use std::str::FromStr;

use anyhow::Ok;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::circuits::zk_signature3::manager::SimplePsyZKSignatureManager;
use psy_config::network_constants::COORDINATOR_USER_TREE_HEIGHT;
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::zk::{data::ZKPublicKeyInfo, wallet::SimplePsyPrivateKey},
};
use psy_data::{config::store_config::PsyHasher, qdata::realm_status::BasicRealmStatus};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QRegisterUserRPCRequest,
};
use psy_store::{
    node::coordinator::{PsyCoordinatorStoreReaderAsync, PsyCoordinatorStoreWriterAsyncImm},
    store,
    store::{backend::LmdbxConfig, journal::JournalStore, Backend, PsyStore},
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;

pub async fn run() -> anyhow::Result<()> {
    let backend: Backend = Backend::Lmdbx(LmdbxConfig {
        lmdbx_path: "realm-status".to_string(),
        lmdbx_mmap_size_gb: 100,
    });
    let psy_store = store::from_backend(backend).await?;
    let psy_store = JournalStore::new(psy_store);

    let realm_ids = (0..1 << COORDINATOR_USER_TREE_HEIGHT).collect::<Vec<u64>>();
    let realm_statuses = (0..1 << COORDINATOR_USER_TREE_HEIGHT)
        .map(|i| BasicRealmStatus {
            checkpoint_id: i,
            realm_root_hash: QHashOut::<F>::rand(),
        })
        .collect::<Vec<BasicRealmStatus<F>>>();

    tracing::info!("set_realm_statuses start");
    let set_start = Instant::now();
    psy_store.set_realm_statuses(&realm_ids, &realm_statuses).await?;
    tracing::info!("set_realm_statuses end, cost: {:?}", set_start.elapsed());

    tracing::info!("get_realm_statuses start");
    let get_start = Instant::now();
    let readed_realm_statuses = psy_store.get_realm_statuses(&realm_ids).await?;
    tracing::info!("get_realm_statuses end, cost: {:?}", get_start.elapsed());

    assert_eq!(realm_statuses, readed_realm_statuses);

    Ok(())
}
