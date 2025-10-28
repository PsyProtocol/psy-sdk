use std::sync::Arc;

use anyhow::Result;
use psy_core::job::id::{LayerId, QProvingJobDataID};
use psy_store::queue::{QueueId, RsmqQueue};

use crate::watcher::{
    common::get_queue_name,
    events::{
        BackupProofEvent, BackupWitnessEvent, JobCompletedEvent, JobStartedEvent, JobTimeoutEvent, UserDeployContractEvent,
        UserDeployContractMetadata, UserGutaSubmissionEvent, UserGutaSubmissionMetadata, UserRegistrationEvent, WatcherMessage,
    },
    watcher_service::{current_datetime, current_timestamp, current_timestamp_mills},
};

pub struct WatcherClient {
    rsmq: Arc<RsmqQueue>,
    queue_id: QueueId,
    node_id: Option<String>,
}

impl WatcherClient {
    pub async fn new(redis_url: &str, pool_size: usize, biz_key: &str, node_id: Option<&str>) -> Result<Self> {
        let queue_name = get_queue_name(biz_key);
        let rsmq = Arc::new(RsmqQueue::new(redis_url, pool_size, &queue_name).await?);

        let queue_id = QueueId::WatcherEvent { queue_biz_key: queue_name };

        rsmq.create_queue_if_not_exists(&queue_id).await?;

        Ok(Self {
            rsmq,
            queue_id,
            node_id: node_id.map(|s| s.to_string()),
        })
    }

    pub async fn get_node_id(&self) -> Option<String> {
        self.node_id.clone()
    }

    pub async fn send_event(&self, event: WatcherMessage) -> Result<()> {
        let serialized = bincode::serialize(&event)?;
        self.rsmq.send_message(&self.queue_id, serialized).await?;
        Ok(())
    }

    // Convenience methods
    pub async fn register_user(&self, public_key: &str) -> Result<()> {
        self.send_event(WatcherMessage::UserRegistration(UserRegistrationEvent {
            public_key: public_key.to_string(),
            timestamp: current_datetime(),
        }))
        .await
    }

    pub async fn deploy_contract(&self, deployer: &str, metadata: UserDeployContractMetadata) -> Result<()> {
        self.send_event(WatcherMessage::DeployContract(UserDeployContractEvent {
            deployer: deployer.to_string(),
            metadata,
            timestamp: current_datetime(),
        }))
        .await
    }

    pub async fn submit_guta(&self, realm_id: u64, metadata: UserGutaSubmissionMetadata) -> Result<()> {
        self.send_event(WatcherMessage::GutaSubmission(UserGutaSubmissionEvent {
            realm_id,
            metadata,
            timestamp: current_datetime(),
        }))
        .await
    }

    //note: when job is started, it means the job have been requested to a worker
    pub async fn report_job_started(&self, job_id: QProvingJobDataID, worker_id: &str, layer_id: LayerId) -> Result<()> {
        self.send_event(WatcherMessage::JobStarted(JobStartedEvent {
            job_id,
            worker_id: worker_id.to_string(),
            start_time: current_timestamp_mills(),
            layer_id,
        }))
        .await
    }

    pub async fn report_job_timeout(&self, job_id: QProvingJobDataID, worker_id: Option<String>, start_time: u64, timeout_time: u64) -> Result<()> {
        self.send_event(WatcherMessage::JobTimeout(JobTimeoutEvent {
            job_id,
            worker_id,
            start_time,
            timeout_time,
        }))
        .await
    }

    pub async fn report_job_completed(&self, job_id: QProvingJobDataID, worker_id: Option<String>, start_time: u64, duration_ms: u64) -> Result<()> {
        self.send_event(WatcherMessage::JobCompleted(JobCompletedEvent {
            job_id,
            worker_id,
            start_time,
            end_time: start_time + duration_ms,
            duration_ms,
        }))
        .await
    }
}
