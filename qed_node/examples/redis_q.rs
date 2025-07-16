/*use kvq::traits::KVQSerializable;
use qed_core::{job::drain_queue::{CheckpointDrainQueueConsumerSyncImm, CheckpointDrainQueueEmitterSyncImm, DrainQueueMetadata, DrainQueueMetadataTagged}, utils::debug_timer::DebugTimer};
use qed_store::queue::drain_queue_redis::dq_imm::DrainQueueRedis;
use rand::{thread_rng, Rng};
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
struct TestItem {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}
impl TestItem {
    pub fn gen_many(a_value: u64, count: usize) -> Vec<Self> {
       (0..count).map(|i| Self{
            a: a_value,
            b: i as u64,
            c: thread_rng().gen::<u64>(),
            d: thread_rng().gen::<u64>(),
        }).collect()
    }
}
impl KVQSerializable for TestItem {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut data = Vec::with_capacity(32);

        data.extend_from_slice(&u64::to_be_bytes(self.a));
        data.extend_from_slice(&u64::to_be_bytes(self.b));
        data.extend_from_slice(&u64::to_be_bytes(self.c));
        data.extend_from_slice(&u64::to_be_bytes(self.d));

        Ok(data)
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.len() != 32 {
            anyhow::bail!("expected 32 bytes when deserializing TestItem, got {}", bytes.len());
        }
        let a = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let b = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let c = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
        let d = u64::from_be_bytes(bytes[24..32].try_into().unwrap());

        Ok(Self {
            a,
            b,
            c,
            d,
        })
    }
}
impl DrainQueueMetadataTagged for TestItem {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {
        DrainQueueMetadata {
            channel_id: 1337,
            checkpoint_id: self.a,
            item_id: self.b,
        }
    }
}


fn run_test_1() -> anyhow::Result<()> {

    let mut timer = DebugTimer::new("dq_rust_1");
    timer.lap("start");
    let q = DrainQueueRedis::new("redis://localhost:6379/0")?;
    timer.lap("connected to redis");

    let items = TestItem::gen_many(1, 10000);
    timer.lap("generated items");
    for item in items {
        q.cdq_push_imm_sync(item)?;
    }
    timer.lap("pushed items");

    let drained_items = q.cdq_drain_imm_sync::<TestItem>(1337, 1)?;
    timer.lap("drained items");

    println!("drained_items: {:?}",drained_items);



    Ok(())




}
fn main() {
    run_test_1().unwrap();
    
}*/

fn main() {
    
}