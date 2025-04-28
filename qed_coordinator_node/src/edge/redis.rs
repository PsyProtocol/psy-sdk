use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use fred::clients::Client;
use fred::prelude::{ClientLike, Config, EventInterface, KeysInterface, PubsubInterface, ReconnectPolicy};
use fred::types::Message;
use qed_node::nimpl::worker_queue_redis::redis_queue::CPQueueNotification;
use crate::context::get_global_redis_pool;

const CP2CE_BROADCAST_CHANNEL: &str = "checkpoint_edge_broadcast_channel";


pub async fn create_pubsub_client(redis_url: &str) -> anyhow::Result<Client> {

    let config = Config::from_url(redis_url)?;

    let perf_config = None;
    let connection_config = None;
    let reconnect_policy = Some(ReconnectPolicy::default());

    let client = Client::new(config, perf_config, connection_config, reconnect_policy);


    client.connect();
    client.wait_for_connect().await?;

    Ok(client)
}

pub async fn publish_checkpoint_sync(payload: String) -> anyhow::Result<()> {
    let pool = get_global_redis_pool()?;
    let conn = pool.next();

    conn.publish(CP2CE_BROADCAST_CHANNEL, payload).await?;

    Ok(())
}


// pub async fn broadcast_checkpoint_sync(notification: CPQueueNotification) -> anyhow::Result<()> {
//     let payload = serde_json::to_vec(&notification)?;
// 
//     let pool = get_global_redis_pool()?;
//     let conn = pool.clone();
// 
//     conn.publish(CP2CE_BROADCAST_CHANNEL, payload).await?;
// 
//     Ok(())
// }
pub async fn broadcast_checkpoint_sync(notification: CPQueueNotification) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(&notification)?;
    let payload_len = payload.len();
    let pool = get_global_redis_pool()?;
    let client = pool.next();

    client.publish(CP2CE_BROADCAST_CHANNEL, payload).await?;
    tracing::info!(
        "✅ Successfully broadcast checkpoint sync notification to channel '{}'. Payload len: {} bytes",
        CP2CE_BROADCAST_CHANNEL,
        payload_len
    );
    Ok(())
}

pub async fn subscribe_checkpoint_sync(
    pubsub_client: Client,
    handler: Arc<dyn Fn(CPQueueNotification) + Send + Sync>,
) -> anyhow::Result<()> {
    pubsub_client.on_message(move |msg: Message| {
        let handler = handler.clone();
        async move {
            if let Some(bytes) = msg.value.as_bytes() {
                match serde_json::from_slice::<CPQueueNotification>(bytes) {
                    Ok(notification) => {
                        handler(notification);
                    }
                    Err(e) => {
                        tracing::warn!("❌ Failed to parse CPQueueNotification: {:?}", e);
                    }
                }
            } else {
                tracing::warn!("⚠️ PubSub received non-bytes payload: {:?}", msg.value);
            }
            Ok(())
        }
    });

    pubsub_client.subscribe(&[CP2CE_BROADCAST_CHANNEL]).await?;

    Ok(())
}

pub fn spawn_fixed_checkpoint_sender() {
    tokio::spawn(async move {
        let checkpoint_id = 1;
        let max_retries = 12;
        let interval = Duration::from_secs(5);

        for i in 0..max_retries {
            tokio::time::sleep(interval).await;

            match broadcast_checkpoint_sync(CPQueueNotification::StartSync { checkpoint: checkpoint_id }).await {
                Ok(_) => {
                    tracing::info!("✅ Broadcasted StartSync (attempt {}/{})", i + 1, max_retries);
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to broadcast StartSync (attempt {}/{}): {:?}", i + 1, max_retries, e);
                }
            }
        }

        tracing::info!("✅ Fixed checkpoint sender finished after {} attempts", max_retries);
    });
}