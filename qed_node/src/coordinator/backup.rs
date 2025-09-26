use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CheckpointBackup {
    pub checkpoint_id: u64,
    pub timestamp: u64,
    pub pair_to_set: Vec<(Vec<u8>, Vec<u8>)>,
    pub removed_keys: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryInfo {
    pub latest_checkpoint: u64,
    pub last_update_timestamp: u64,
    pub checkpoints_available: Vec<u64>,
}

#[derive(Clone)]
pub struct S3BackupClient {
    bucket: String,
    client: aws_sdk_s3::Client,
}

impl S3BackupClient {
    pub async fn new(bucket: String) -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_s3::Client::new(&config);
        info!("Testing S3 connection...");
        let result = client.head_bucket().bucket(&bucket).send().await;
        match result {
            Ok(_) => {
                info!("✅ S3 connection successful, bucket '{}' is accessible", bucket);
            }
            Err(e) => {
                error!("❌ S3 connection failed: {}", e);
                return Err(e.into());
            }
        }
        Ok(Self { bucket, client })
    }

    pub async fn new_from_env() -> Result<Self> {
        let bucket = std::env::var("QED_BACKUP_BUCKET").context("QED_BACKUP_BUCKET environment variable not set")?;
        Self::new(bucket).await
    }

    pub async fn backup_checkpoint(&self, backup: &CheckpointBackup) -> Result<()> {
        let key = self.get_changes_key(backup.checkpoint_id);
        let data = serde_json::to_vec(backup).context("Failed to serialize checkpoint backup")?;

        match self
            .client
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
                error!("❌ Failed to backup checkpoint {} to S3: {:?}", backup.checkpoint_id, e);
                Err(e.into())
            }
        }
    }

    pub async fn fetch_checkpoint_backup(&self, checkpoint_id: u64) -> Result<CheckpointBackup> {
        let key = self.get_changes_key(checkpoint_id);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch checkpoint backup from S3")?;

        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();

        let backup: CheckpointBackup = serde_json::from_slice(&data).context("Failed to deserialize checkpoint backup")?;

        Ok(backup)
    }

    pub async fn fetch_recovery_info(&self) -> Result<RecoveryInfo> {
        let key = self.get_recovery_info_key();

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch recovery info from S3")?;

        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();

        let recovery_info: RecoveryInfo = serde_json::from_slice(&data).context("Failed to deserialize recovery info")?;

        Ok(recovery_info)
    }

    pub async fn list_available_checkpoints(&self) -> Result<Vec<u64>> {
        let prefix = self.get_changes_prefix_key();

        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .send()
            .await
            .context("Failed to list checkpoints from S3")?;

        let mut checkpoints = Vec::new();
        for object in response.contents() {
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

        checkpoints.sort();
        Ok(checkpoints)
    }

    async fn update_recovery_info(&self, latest_checkpoint: u64) -> Result<()> {
        let mut recovery_info = self.fetch_recovery_info().await.unwrap_or_else(|_| RecoveryInfo {
            latest_checkpoint: 0,
            last_update_timestamp: 0,
            checkpoints_available: vec![],
        });

        recovery_info.latest_checkpoint = latest_checkpoint;
        recovery_info.last_update_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        if !recovery_info.checkpoints_available.contains(&latest_checkpoint) {
            recovery_info.checkpoints_available.push(latest_checkpoint);
            recovery_info.checkpoints_available.sort();
        }

        let key = self.get_recovery_info_key();
        let data = serde_json::to_vec(&recovery_info).context("Failed to serialize recovery info")?;

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

    fn get_changes_prefix_key(&self) -> String {
        format!("coordinator/changes/")
    }

    fn get_changes_key(&self, checkpoint_id: u64) -> String {
        format!("coordinator/changes/{}.json", checkpoint_id)
    }

    fn get_recovery_info_key(&self) -> String {
        format!("coordinator/recovery_info.json")
    }
}

pub async fn create_checkpoint_backup(
    checkpoint_id: u64,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
) -> Result<CheckpointBackup> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    Ok(CheckpointBackup {
        checkpoint_id,
        timestamp,
        pair_to_set: journal_changes,
        removed_keys,
    })
}

pub async fn try_backup_coordinator_checkpoint(
    backup_client: &S3BackupClient,
    checkpoint_id: u64,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
) {
    match create_checkpoint_backup(checkpoint_id, journal_changes, removed_keys).await {
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
