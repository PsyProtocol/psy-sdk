
use kvq::memory::{arc_imm::KVQArcImmutableStoreWrapper, simple::KVQSimpleMemoryBackingStore};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_data::{protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints, qblock::cmds::{core::QEDBlockCommands, register_user::QBCRegisterUser}};
use qed_store::{qblock::process::simple::SimpleBlockProcessor, traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync}};

type GF = GoldilocksField;


fn test_simple_block_processor() -> anyhow::Result<()> {
    let mut t = DebugTimer::new("test_kvq_simple_store_arc");
    t.lap("start");
    let st = KVQArcImmutableStoreWrapper::<KVQSimpleMemoryBackingStore>::new(KVQSimpleMemoryBackingStore::new());
    let cur_checkpoint = st.initialize_store()?;
    t.event(format!("current_checkpoint: {}", cur_checkpoint));

    let circuit_fingerprints = QEDWorkerToolboxCoreCircuitFingerprints::default();


    let block_0_cmds = QEDBlockCommands::<GF>{
        register_users: vec![
            QBCRegisterUser::new(QHashOut::from_values(1,2,3,4)),
            QBCRegisterUser::new(QHashOut::from_values(5,6,7,8)),
            QBCRegisterUser::new(QHashOut::from_values(13371,13372,13373,13374)),
        ],
        deploy_contracts: vec![],
        update_users: vec![],
    };
    SimpleBlockProcessor::process_block(&st, &block_0_cmds, &circuit_fingerprints)?;
    let latest_block_st = st.get_latest_l2_block_state()?;
    println!("latest_block_st: {:?}",latest_block_st);
    Ok(())

}


fn test_block_combos() -> anyhow::Result<()>{
    test_simple_block_processor()?;
    Ok(())

}

fn main() {


    test_block_combos().unwrap_or_else(|e| {
        eprintln!("Error: {:?}", e);
    })

}