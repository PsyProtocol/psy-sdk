use anyhow::Result;
use qed_core::utils::debug_timer::DebugTimer;
use scylla::{Session, SessionBuilder};
use std::env;
use std::sync::Arc;

use tokio::sync::Semaphore;

#[tokio::main]
async fn main() -> Result<()> {
    let uri = env::var("SCYLLA_URI").unwrap_or_else(|_| "127.0.0.1:9042".to_string());

    let mut timer = DebugTimer::new("syclla_test");
    timer.lap("connecting");

    println!("Connecting to {} ...", uri);

    let session: Session = SessionBuilder::new().known_node(uri).build().await?;
    let session = Arc::new(session);

    session.query_unpaged("CREATE KEYSPACE IF NOT EXISTS examples_ks WITH REPLICATION = {'class' : 'NetworkTopologyStrategy', 'replication_factor' : 1}", &[]).await?;

    session
        .query_unpaged(
"CREATE TABLE IF NOT EXISTS examples_ks.merkle_store_a ( tree_id smallint, primary_id bigint, secondary_id bigint, node_level smallint, node_index bigint, checkpoint_id bigint, node_value blob, PRIMARY KEY ((tree_id, primary_id, secondary_id, node_level, node_index), checkpoint_id) ) WITH CLUSTERING ORDER BY (checkpoint_id DESC)",
            //"CREATE TABLE IF NOT EXISTS examples_ks.parallel_prepared (a int, b int, c text, primary key (a, b))",
            &[],
        )
        .await?;

    let prepared = Arc::new(
        session
            .prepare("INSERT INTO examples_ks.parallel_prepared (a, b, c) VALUES (?, ?, 'abc')")
            .await?,
    );
    println!("Prepared statement: {:#?}", prepared);
    timer.lap("created prepared statement");

    let parallelism = 256;
    let sem = Arc::new(Semaphore::new(parallelism));
    let mut timer_100k = DebugTimer::new("syclla_test_100k_timer");
    timer_100k.lap("started processing 100k inserts");

    for i in 0..100_000usize {
        if i % 1000 == 0 {
            println!("{}", i);
            timer.lap("processed 1k insertions");
        }
        let session = session.clone();
        let prepared = prepared.clone();
        let permit = sem.clone().acquire_owned().await;
        tokio::task::spawn(async move {
            session
                .execute_unpaged(&prepared, (i as i32, 2 * i as i32))
                .await
                .unwrap();

            let _permit = permit;
        });
    }

    // Wait for all in-flight requests to finish
    for _ in 0..parallelism {
        sem.acquire().await.unwrap().forget();
    }

    timer_100k.lap("finisehd 100k insertions");
    println!("Ok.");

    Ok(())
}