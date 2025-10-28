
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use std::sync::Arc;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use psy_data::{protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints, qblock::cmds::{core::QEDBlockCommands, register_user::QBCRegisterUser}};
use psy_data::{qblock::process::simple::SimpleBlockProcessor, traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync}};
use psy_store::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;

type GF = GoldilocksField;


async fn test_simple_block_processor() -> anyhow::Result<()> {
    let mut t = DebugTimer::new("test_kvq_simple_store_arc");
    t.lap("start");
    let st = Arc::new(KVQSimpleMemoryBackingStore::new());
    use psy_core::data::qhashout::QHashOut;
    let cur_checkpoint = st.initialize_store(None).await?;
    t.event(format!("current_checkpoint: {}", cur_checkpoint));

    let circuit_fingerprints = QEDWorkerToolboxCoreCircuitFingerprints::default();


    let block_0_cmds = QEDBlockCommands::<GF>{
        register_users: vec![
            QBCRegisterUser::new_from_u64s([1;4], [13371, 13372, 13373, 13374]),
            QBCRegisterUser::new_from_u64s([1;4], [13375, 13376, 13377, 13378]),
            QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
            QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
        ],
        deploy_contracts: vec![],
        update_users: vec![],
    };
    SimpleBlockProcessor::process_block(&st, &block_0_cmds, &circuit_fingerprints).await?;
    let latest_block_st = st.get_latest_l2_block_state().await?;
    println!("latest_block_st: {:?}",latest_block_st);
    Ok(())

}


async fn test_block_combos() -> anyhow::Result<()>{
    test_simple_block_processor().await?;
    Ok(())

}

#[tokio::main]
async fn main() {


    test_block_combos().await.unwrap_or_else(|e| {
        eprintln!("Error: {:?}", e);
    })

}
