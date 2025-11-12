use std::{
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_store::queue::redis_queue::QueueOffsetState;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::coordinator::backup::RecoveryInfo;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RealmCheckpointBackup {
    pub checkpoint_id: u64,
    pub realm_id: u32,
    pub timestamp: u64,
    pub pair_to_set: Vec<(Vec<u8>, Vec<u8>)>,
    pub removed_keys: Vec<Vec<u8>>,
    pub is_committed: bool,
}

type F = psy_data::config::store_config::PsyFelt;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RealmPendingUsersBackup {
    pub checkpoint_id: u64,
    pub realm_id: u32,
    pub timestamp: u64,
    pub start_user_index: u64,
    pub pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RealmStateBackup {
    pub checkpoint_id: u64,
    pub realm_id: u32,
    pub timestamp: u64,
    pub last_processed_user_num: u64,
    pub current_processed_user_num: u64,
    pub total_pending_users: u64,
    pub is_committed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RealmRecoveryInfo {
    pub realm_id: u32,
    pub latest_checkpoint: u64,
    pub last_update_timestamp: u64,
    pub checkpoints_available: Vec<u64>,
    pub cached_checkpoint_available: Vec<u64>,
}

#[derive(Clone)]
pub struct RealmS3BackupClient {
    pub realm_id: u32,
    pub bucket: String,
    pub client: aws_sdk_s3::Client,
    pub recovery_info: Arc<RwLock<RecoveryInfo>>,
}

impl RealmS3BackupClient {
    pub async fn new(realm_id: u32, bucket: String) -> Result<Self> {
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

        let backup_client = Self {
            realm_id,
            bucket,
            client,
            recovery_info: Arc::new(RwLock::new(RecoveryInfo::default())),
        };
        if let Ok(recovery_info) = backup_client.fetch_recovery_info().await {
            let mut local_recovery_info = backup_client
                .recovery_info
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to acquire write lock"))?;
            *local_recovery_info = RecoveryInfo {
                latest_checkpoint: recovery_info.latest_checkpoint,
                last_update_timestamp: recovery_info.last_update_timestamp,
                checkpoints_available: recovery_info.checkpoints_available,
                cached_checkpoint_available: recovery_info.cached_checkpoint_available,
            };
        };
        Ok(backup_client)
    }

    pub async fn new_from_env(realm_id: u32) -> Result<Self> {
        let bucket = std::env::var("PSY_BACKUP_BUCKET").context("PSY_BACKUP_BUCKET environment variable not set")?;
        Self::new(realm_id, bucket).await
    }

    pub fn find_last_available_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64> {
        let recovery_info = self.recovery_info.read().map_err(|_| anyhow::anyhow!("Failed to acquire read lock"))?;
        recovery_info
            .checkpoints_available
            .binary_search(&checkpoint_id)
            .map(|idx| recovery_info.checkpoints_available[idx])
            .or_else(|pos| {
                if pos == 0 {
                    Err(anyhow::anyhow!("No available checkpoint before {}", checkpoint_id))
                } else {
                    Ok(recovery_info.checkpoints_available[pos - 1])
                }
            })
    }

    pub fn find_last_available_cached_checkpoint(&self, checkpoint_id: u64) -> anyhow::Result<u64> {
        let recovery_info = self.recovery_info.read().map_err(|_| anyhow::anyhow!("Failed to acquire read lock"))?;
        recovery_info
            .cached_checkpoint_available
            .binary_search(&checkpoint_id)
            .map(|idx| recovery_info.cached_checkpoint_available[idx])
            .or_else(|pos| {
                if pos == 0 {
                    Err(anyhow::anyhow!("No available cached checkpoint before {}", checkpoint_id))
                } else {
                    Ok(recovery_info.cached_checkpoint_available[pos - 1])
                }
            })
    }

    pub async fn backup_checkpoint(&self, backup: &RealmCheckpointBackup) -> Result<()> {
        let key = self.get_changes_key(backup.checkpoint_id);
        let data = serde_json::to_vec(backup).context("Failed to serialize realm checkpoint backup")?;

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
                info!("✅ Backup realm {} checkpoint {} to S3: {}", self.realm_id, backup.checkpoint_id, key);

                // Update recovery info (best effort, don't fail if this fails)
                if let Err(e) = self.update_recovery_info(backup.checkpoint_id, backup.is_committed).await {
                    warn!("Failed to update realm recovery info: {}", e);
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Failed to backup realm {} checkpoint {} to S3: {:?}",
                    self.realm_id, backup.checkpoint_id, e
                );
                Err(e.into())
            }
        }
    }

    pub async fn backup_pending_users(&self, backup: &RealmPendingUsersBackup) -> Result<()> {
        for (i, user) in backup.pending_users.iter().enumerate() {
            let key = self.get_pending_users_key(backup.start_user_index + i as u64);
            let data = serde_json::to_vec(user).context("Failed to serialize realm pending users backup")?;

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
                    info!(
                        "✅ Backup realm {} pending users {} to S3: {}",
                        self.realm_id,
                        backup.start_user_index + i as u64,
                        key
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Failed to backup realm {} pending users {} to S3: {:?}",
                        self.realm_id, backup.checkpoint_id, e
                    );
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    pub async fn backup_pending_users_queue_state(&self, backup: &QueueOffsetState) -> Result<()> {
        let key = self.get_pending_users_queue_state_key(backup.checkpoint_id);
        let data = serde_json::to_vec(backup).context("Failed to serialize realm pending users queue state backup")?;

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
                info!(
                    "✅ Backup realm {} pending users queue state at checkpoint {} to S3: {}",
                    self.realm_id, backup.checkpoint_id, key
                );
            }
            Err(e) => {
                error!(
                    "❌ Failed to backup realm {} pending users {} to S3: {:?}",
                    self.realm_id, backup.checkpoint_id, e
                );
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn backup_realm_state(&self, backup: &RealmStateBackup) -> Result<()> {
        let key = self.get_realm_state_key(backup.checkpoint_id);
        let data = serde_json::to_vec(backup).context("Failed to serialize realm state backup")?;

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
                info!("✅ Backup realm {} state {} to S3: {}", self.realm_id, backup.checkpoint_id, key);
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Failed to backup realm {} state {} to S3: {:?}",
                    self.realm_id, backup.checkpoint_id, e
                );
                return Err(e.into());
            }
        }
    }

    pub async fn fetch_checkpoint_backup(&self, checkpoint_id: u64) -> Result<RealmCheckpointBackup> {
        let key = self.get_changes_key(checkpoint_id);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch realm checkpoint backup from S3")?;

        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();

        let backup: RealmCheckpointBackup = serde_json::from_slice(&data).context("Failed to deserialize realm checkpoint backup")?;

        Ok(backup)
    }

    pub async fn fetch_recovery_info(&self) -> Result<RealmRecoveryInfo> {
        let key = self.get_recovery_info_key();

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch realm recovery info from S3")?;

        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();

        let recovery_info: RealmRecoveryInfo = serde_json::from_slice(&data).context("Failed to deserialize realm recovery info")?;

        Ok(recovery_info)
    }

    pub async fn fetch_realm_state(&self, checkpoint_id: u64) -> Result<RealmStateBackup> {
        let checkpoint_id = self.find_last_available_checkpoint(checkpoint_id)?;
        let key = self.get_realm_state_key(checkpoint_id);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch realm state backup from S3")?;

        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();
        let backup: RealmStateBackup = serde_json::from_slice(&data).context("Failed to deserialize realm state backup")?;
        Ok(backup)
    }

    pub async fn fetch_pending_users(&self, start: u64, end: u64) -> Result<Vec<MerkleProofCore<QHashOut<F>>>> {
        let mut pending_users = vec![];
        for user in start..end {
            let key = self.get_pending_users_key(user);

            let response = self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(&key)
                .send()
                .await
                .context("Failed to fetch realm pending users backup from S3")?;
            let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();
            let backup: MerkleProofCore<QHashOut<F>> = serde_json::from_slice(&data).context("Failed to deserialize realm pending users backup")?;
            pending_users.push(backup);
        }
        Ok(pending_users)
    }

    pub async fn fetch_pending_users_queue_state(&self, checkpoint_id: u64) -> Result<QueueOffsetState> {
        let key = self.get_pending_users_queue_state_key(checkpoint_id);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to fetch realm pending users queue state backup from S3")?;
        let data = response.body.collect().await.context("Failed to read S3 response body")?.into_bytes();
        let backup: QueueOffsetState = serde_json::from_slice(&data).context("Failed to deserialize realm pending users backup")?;

        Ok(backup)
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
            .context("Failed to list realm checkpoints from S3")?;

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

    async fn update_recovery_info(&self, latest_checkpoint: u64, is_committed: bool) -> Result<()> {
        let mut recovery_info = self.fetch_recovery_info().await.unwrap_or_else(|_| RealmRecoveryInfo {
            realm_id: self.realm_id,
            latest_checkpoint: 0,
            last_update_timestamp: 0,
            checkpoints_available: vec![],
            cached_checkpoint_available: vec![],
        });

        recovery_info.last_update_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        {
            let mut local_recovery_info = self.recovery_info.write().map_err(|_| anyhow::anyhow!("Failed to acquire write lock"))?;
            if is_committed {
                recovery_info.latest_checkpoint = latest_checkpoint;
                local_recovery_info.latest_checkpoint = latest_checkpoint;
                if !recovery_info.checkpoints_available.contains(&latest_checkpoint) {
                    recovery_info.checkpoints_available.push(latest_checkpoint);
                    recovery_info.checkpoints_available.sort();

                    local_recovery_info.latest_checkpoint = latest_checkpoint;
                    local_recovery_info.checkpoints_available.push(latest_checkpoint);
                    local_recovery_info.checkpoints_available.sort();
                    local_recovery_info.last_update_timestamp = recovery_info.last_update_timestamp;
                }
            } else {
                if !recovery_info.cached_checkpoint_available.contains(&latest_checkpoint) {
                    recovery_info.cached_checkpoint_available.push(latest_checkpoint);
                    recovery_info.cached_checkpoint_available.sort();

                    local_recovery_info.cached_checkpoint_available.push(latest_checkpoint);
                    local_recovery_info.cached_checkpoint_available.sort();
                }
            }
        }

        let key = self.get_recovery_info_key();
        let data = serde_json::to_vec(&recovery_info).context("Failed to serialize realm recovery info")?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .content_type("application/json")
            .send()
            .await
            .context("Failed to upload realm recovery info to S3")?;

        Ok(())
    }

    fn get_changes_prefix_key(&self) -> String {
        format!("realm/{}/changes/", self.realm_id)
    }

    fn get_changes_key(&self, checkpoint_id: u64) -> String {
        format!("realm{}/changes/{}.json", self.realm_id, checkpoint_id)
    }

    fn get_pending_users_key(&self, user_index: u64) -> String {
        format!("realm{}/users/{}.json", self.realm_id, user_index)
    }

    fn get_pending_users_queue_state_key(&self, checkpoint_id: u64) -> String {
        format!("realm{}/users_queue_state/{}.json", self.realm_id, checkpoint_id)
    }

    fn get_realm_state_key(&self, checkpoint_id: u64) -> String {
        format!("realm{}/states/{}.json", self.realm_id, checkpoint_id)
    }

    fn get_recovery_info_key(&self) -> String {
        format!("realm{}/recovery_info.json", self.realm_id)
    }
}

pub fn create_realm_checkpoint_backup(
    realm_id: u32,
    checkpoint_id: u64,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
    is_committed: bool,
) -> RealmCheckpointBackup {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    RealmCheckpointBackup {
        realm_id,
        checkpoint_id,
        timestamp,
        pair_to_set: journal_changes,
        removed_keys,
        is_committed,
    }
}

pub fn create_realm_pending_users_backup(
    realm_id: u32,
    checkpoint_id: u64,
    start_user_index: u64,
    pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
) -> RealmPendingUsersBackup {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    RealmPendingUsersBackup {
        realm_id,
        checkpoint_id,
        timestamp,
        start_user_index,
        pending_users,
    }
}

pub fn create_realm_state_backup(
    realm_id: u32,
    checkpoint_id: u64,
    last_processed_user_num: u64,
    current_processed_user_num: u64,
    total_pending_users: u64,
    is_committed: bool,
) -> RealmStateBackup {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    RealmStateBackup {
        realm_id,
        checkpoint_id,
        timestamp,
        last_processed_user_num,
        current_processed_user_num,
        total_pending_users,
        is_committed,
    }
}

pub async fn try_backup_realm_checkpoint(
    backup_client: &RealmS3BackupClient,
    checkpoint_id: u64,
    journal_changes: Vec<(Vec<u8>, Vec<u8>)>,
    removed_keys: Vec<Vec<u8>>,
    is_committed: bool,
) {
    let backup = create_realm_checkpoint_backup(backup_client.realm_id, checkpoint_id, journal_changes, removed_keys, is_committed);
    if let Err(e) = backup_client.backup_checkpoint(&backup).await {
        error!("❌ Realm backup failed for checkpoint {}: {}", checkpoint_id, e);
    }
}
