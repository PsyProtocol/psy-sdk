use anyhow::{Context, Result};
use qed_core::job::id::QProvingJobDataID;
use qed_data::{config::store_config::QEDFelt, qdata::checkpoint::QEDL2BlockState};
use serde::{Deserialize, Serialize};
use tokio::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

type F = QEDFelt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CheckpointBackup {
    pub checkpoint_id: u64,
    pub timestamp: u64,
    pub l2_block_state: QEDL2BlockState<F>,
    pub journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    pub pending_tasks: Vec<QProvingJobDataID>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryInfo {
    pub latest_checkpoint: u64,
    pub network: String,
    pub node_type: String,
    pub last_update_timestamp: u64,
    pub checkpoints_available: Vec<u64>,
}

pub struct S3BackupClient {
    bucket: String,
    network: String,
    node_type: String,
    client: aws_sdk_s3::Client,
}

impl S3BackupClient {
    pub async fn new() -> Result<Self> {
        let bucket = std::env::var("QED_BACKUP_BUCKET")
            .context("QED_BACKUP_BUCKET environment variable not set")?;
        let network = std::env::var("QED_NETWORK")
            .unwrap_or_else(|_| "mainnet".to_string());
        
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        
        Ok(Self {
            bucket,
            network,
            node_type: "coordinator-processor".to_string(),
            client,
        })
    }

    pub async fn backup_checkpoint(&self, backup: &CheckpointBackup) -> Result<()> {
        let key = self.get_checkpoint_key(backup.checkpoint_id);
        let data = serde_json::to_vec(backup)
            .context("Failed to serialize checkpoint backup")?;

        match self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type("application/json")
            .send()
            .await
        {
            Ok(_) => {
                info!("✅ Backup checkpoint {} to S3: {}", backup.checkpoint_id, key);
                
                // Update recovery info (best effort, don't fail if this fails)
                if let Err(e) = self.update_recovery_info(backup.checkpoint_id).await {
                    warn!("Failed to update recovery info: {}", e);
                }
                
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to backup checkpoint {} to S3: {}", backup.checkpoint_id, e);
                Err(e.into())
            }
        }
    }

    pub async fn fetch_checkpoint_backup(&self, checkpoint_id: u64) -> Result<CheckpointBackup> {
        let key = self.get_checkpoint_key(checkpoint_id);
        
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch checkpoint backup from S3")?;

        let data = response.body.collect().await
            .context("Failed to read S3 response body")?
            .into_bytes();

        let backup: CheckpointBackup = serde_json::from_slice(&data)
            .context("Failed to deserialize checkpoint backup")?;

        Ok(backup)
    }

    pub async fn fetch_recovery_info(&self) -> Result<RecoveryInfo> {
        let key = self.get_recovery_info_key();
        
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch recovery info from S3")?;

        let data = response.body.collect().await
            .context("Failed to read S3 response body")?
            .into_bytes();

        let recovery_info: RecoveryInfo = serde_json::from_slice(&data)
            .context("Failed to deserialize recovery info")?;

        Ok(recovery_info)
    }

    pub async fn list_available_checkpoints(&self) -> Result<Vec<u64>> {
        let prefix = format!("{}/{}/checkpoints/", self.network, self.node_type);
        
        let response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await
            .context("Failed to list checkpoints from S3")?;

        let mut checkpoints = Vec::new();
        if let Some(contents) = response.contents() {
            for object in contents {
                if let Some(key) = object.key() {
                    if let Some(filename) = key.split('/').last() {
                        if let Some(checkpoint_str) = filename.strip_suffix(".json") {
                            if let Ok(checkpoint_id) = checkpoint_str.parse::<u64>() {
                                checkpoints.push(checkpoint_id);
                            }
                        }
                    }
                }
            }
        }

        checkpoints.sort();
        Ok(checkpoints)
    }

    async fn update_recovery_info(&self, latest_checkpoint: u64) -> Result<()> {
        let mut recovery_info = self.fetch_recovery_info().await
            .unwrap_or_else(|_| RecoveryInfo {
                latest_checkpoint: 0,
                network: self.network.clone(),
                node_type: self.node_type.clone(),
                last_update_timestamp: 0,
                checkpoints_available: vec![],
            });

        recovery_info.latest_checkpoint = latest_checkpoint;
        recovery_info.last_update_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if !recovery_info.checkpoints_available.contains(&latest_checkpoint) {
            recovery_info.checkpoints_available.push(latest_checkpoint);
            recovery_info.checkpoints_available.sort();
        }

        let key = self.get_recovery_info_key();
        let data = serde_json::to_vec(&recovery_info)
            .context("Failed to serialize recovery info")?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type("application/json")
            .send()
            .await
            .context("Failed to upload recovery info to S3")?;

        Ok(())
    }

    fn get_checkpoint_key(&self, checkpoint_id: u64) -> String {
        format!("{}/{}/checkpoints/{}.json", self.network, self.node_type, checkpoint_id)
    }

    fn get_recovery_info_key(&self) -> String {
        format!("{}/{}/recovery/recovery_info.json", self.network, self.node_type)
    }
}

pub async fn create_checkpoint_backup(
    checkpoint_id: u64,
    l2_block_state: &QEDL2BlockState<F>,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    pending_tasks: Vec<QProvingJobDataID>,
) -> Result<CheckpointBackup> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(CheckpointBackup {
        checkpoint_id,
        timestamp,
        l2_block_state: l2_block_state.clone(),
        journal_changes,
        pending_tasks,
    })
}

pub async fn try_backup_checkpoint(
    backup_client: &S3BackupClient,
    checkpoint_id: u64,
    l2_block_state: &QEDL2BlockState<F>,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    pending_tasks: Vec<QProvingJobDataID>,
) {
    match create_checkpoint_backup(checkpoint_id, l2_block_state, journal_changes, pending_tasks).await {
        Ok(backup) => {
            if let Err(e) = backup_client.backup_checkpoint(&backup).await {
                error!("❌ Backup failed for checkpoint {}: {}", checkpoint_id, e);
            }
        }
        Err(e) => {
            error!("❌ Failed to create backup for checkpoint {}: {}", checkpoint_id, e);
        }
    }
}