use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use std::sync::Arc;
use plonky2::hash::poseidon::PoseidonHash;
use qed_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use qed_data::{
    config::store_config::UserTreeStore,
    models::kvq_merkle::model::{
        KVQFixedConfigMerkleTreeModelCore,
        KVQFixedConfigMerkleTreeModelReaderCore,
    },
};

fn test_kvq_simple_store_arc() -> anyhow::Result<()> {
    let mut t = DebugTimer::new("test_kvq_simple_store_arc");
    t.lap("hiii");
    let st = Arc::new(KVQSimpleMemoryBackingStore::new());
    let lf1 = UserTreeStore::set_leaf_fc(&st, 0, 0, QHashOut::from_values(1, 2, 3, 4))?;
    println!("lf2 result: {}", lf1.verify::<PoseidonHash>());

    let lf2 = UserTreeStore::set_leaf_fc(&st, 1, 1, QHashOut::from_values(5, 6, 7, 8))?;
    println!("lf2 result: {}", lf2.verify::<PoseidonHash>());
    t.lap("start modifications");

    for i in 0..100000 {
        let _tmp =
            UserTreeStore::set_leaf_fc(&st, 2, i, QHashOut::from_values(i + 5, 6, 7, 8))?;
    }
    t.batch_average("end modifications", "set_user_leaf", 100000);

    let lf = UserTreeStore::get_leaf_fc(&st, 2, 0)?;

    println!("lf result: {}", lf.verify::<PoseidonHash>());

    println!("{:?}", serde_json::to_string_pretty(&lf)?);

    Ok(())
}

fn test_kvq_simple_store() -> anyhow::Result<()> {
    let mut t = DebugTimer::new("test_kvq_simple_store");
    t.lap("hiii");
    let st = KVQSimpleMemoryBackingStore::new();
    let lf1 = UserTreeStore::set_leaf_fc(&st, 0, 0, QHashOut::from_values(1, 2, 3, 4))?;
    println!("lf2 result: {}", lf1.verify::<PoseidonHash>());

    let lf2 = UserTreeStore::set_leaf_fc(&st, 1, 1, QHashOut::from_values(5, 6, 7, 8))?;
    println!("lf2 result: {}", lf2.verify::<PoseidonHash>());
    t.lap("start modifications");

    for i in 0..100000 {
        let _tmp = UserTreeStore::set_leaf_fc(&st, 2, i, QHashOut::from_values(i + 5, 6, 7, 8))?;
    }
    t.batch_average("end modifications", "set_user_leaf", 100000);

    let lf = UserTreeStore::get_leaf_fc(&st, 2, 0)?;

    println!("lf result: {}", lf.verify::<PoseidonHash>());

    println!("{:?}", serde_json::to_string_pretty(&lf)?);

    Ok(())
}

fn test_combo_kvq_simple() -> anyhow::Result<()> {
    test_kvq_simple_store()?;
    test_kvq_simple_store_arc()?;
    test_kvq_simple_store()?;
    test_kvq_simple_store_arc()?;
    Ok(())
}

fn main() {
    test_combo_kvq_simple().unwrap_or_else(|e| {
        eprintln!("Error: {:?}", e);
    })
}
