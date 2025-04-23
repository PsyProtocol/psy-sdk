use fred::prelude::*;
use kvq::traits::KVQSerializable;
use qed_core::{
    job::drain_queue::{
        CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm,
        DrainQueueMetadata, DrainQueueMetadataTagged,
    },
    utils::debug_timer::DebugTimer,
};
use qed_node::nimpl::drain_queue_fred::DrainQueueFred;
use std::time::Duration;
use tokio::task::JoinHandle;


use rand::{thread_rng, Rng};
use qed_node::nimpl::new_fred_pool;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
struct TestItem {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}
impl TestItem {
    pub fn gen_many(a_value: u64, count: usize, start_idx: usize) -> Vec<Self> {
        (0..count)
            .map(|i| Self {
                a: a_value,
                b: (i+start_idx) as u64,
                c: thread_rng().gen::<u64>(),
                d: thread_rng().gen::<u64>(),
            })
            .collect()
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
            anyhow::bail!(
                "expected 32 bytes when deserializing TestItem, got {}",
                bytes.len()
            );
        }
        let a = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let b = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        let c = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
        let d = u64::from_be_bytes(bytes[24..32].try_into().unwrap());

        Ok(Self { a, b, c, d })
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


async fn run_fred_test3() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;
    
    timer.lap("connected to redis");

    let q = DrainQueueFred::new(pool);

    let worker_count = 16usize;
    let items_per_worker = 2000usize;
    let checkpoint_id = 2u64;

    let res = (0..worker_count).map(|i|{
        let dq = q.clone();
        let start_idx = items_per_worker*i;
        let item_count = items_per_worker;
        let jhandle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

            let items = TestItem::gen_many(checkpoint_id, item_count, start_idx);
            for item in items {
                dq.cdq_push_imm(item).await?;
            }
    
            Ok(())
        });
        jhandle
    }).collect::<Vec<_>>();

    for r in res {
        r.await??;
    }

    timer.lap("pushed items");

    let drained_items = q.cdq_drain_imm::<TestItem>(1337, 2).await?;
    timer.lap("drained items");
    println!("ditems_len: {}", drained_items.len());
    //println!("drained_items: {:?}", drained_items);

    Ok(())
}
async fn run_fred_test2() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;
    
    timer.lap("connected to redis");

    let q = DrainQueueFred::new(pool);

    /* 
    let worker_count = 4usize;
    let items_per_worker = 5000usize;
    let checkpoint_id = 2u64;*/

    let q1 = q.clone();
    let q2 = q.clone();

    let res1: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

        let items = TestItem::gen_many(2, 5000, 0);
        for item in items {
            q1.cdq_push_imm(item).await?;
        }

        Ok(())
    });

    let res2: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {

        let items = TestItem::gen_many(2, 5000, 5000);
        for item in items {
            q2.cdq_push_imm(item).await?;
        }

        Ok(())
    });

    let _ = res1.await??;
    let _ = res2.await??;
    timer.lap("pushed items");

    let drained_items = q.cdq_drain_imm::<TestItem>(1337, 2).await?;
    timer.lap("drained items");
    println!("ditems_len: {}", drained_items.len());
    //println!("drained_items: {:?}", drained_items);

    Ok(())
}


async fn run_fred_test() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("dq_rust_2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;
    
    timer.lap("connected to redis");

    let q = DrainQueueFred::new(pool);
    let items = TestItem::gen_many(2, 10000, 0);
    timer.lap("generated items");
    for item in items {
        q.cdq_push_imm(item).await?;
    }
    timer.lap("pushed items");

    let drained_items = q.cdq_drain_imm::<TestItem>(1337, 2).await?;
    timer.lap("drained items");
    println!("ditems_len: {}", drained_items.len());
    //println!("drained_items: {:?}", drained_items);

    Ok(())
}
#[tokio::main]
async fn main() {
    run_fred_test().await.unwrap();
    run_fred_test2().await.unwrap();
    run_fred_test3().await.unwrap();
}
