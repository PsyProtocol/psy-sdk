use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use psy_core::job::id::{QProvingJobDataID, ProvingJobCircuitType};
use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn, trace};

pub use psy_prover::local::provider::{JobInfo, JobLocation};

type F = GoldilocksField;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmJobData {
    pub id: u32,
    pub checkpoints: IndexMap<u64, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJobTracker {
    pub coordinator: IndexMap<u64, Vec<String>>,
    pub realms: Vec<RealmJobData>,
}

impl WorkerJobTracker {
    pub fn new() -> Self {
        Self {
            coordinator: IndexMap::new(),
            realms: Vec::new(),
        }
    }

    /// Check if a job type is supported for rewards claiming
    fn is_job_type_supported_for_rewards(circuit_type: ProvingJobCircuitType) -> bool {
        matches!(circuit_type,
            // User Registration jobs (type 0)
            ProvingJobCircuitType::AppendUserRegistrationTree |
            ProvingJobCircuitType::AppendUserRegistrationTreeAggregate |
            ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate |

            // GUTA jobs (type 1)
            ProvingJobCircuitType::GUTAOnlyRegisterUsers |
            ProvingJobCircuitType::GUTARegisterUsers |
            ProvingJobCircuitType::GUTATwoEndCap |
            ProvingJobCircuitType::GUTATwoGUTA |
            ProvingJobCircuitType::GUTALeftEndCapRightGUTA |
            ProvingJobCircuitType::GUTALeftGUTARightEndCap |
            ProvingJobCircuitType::GUTASingleEndCap |
            ProvingJobCircuitType::GUTAVerifyToCap |
            ProvingJobCircuitType::GUTANoChange |
            ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade |
            ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade |

            // Contract Deploy jobs (type 2)
            ProvingJobCircuitType::BatchDeployContracts |
            ProvingJobCircuitType::BatchDeployContractsAggregate |
            ProvingJobCircuitType::DummyBatchDeployContractsAggregate
        )
    }

    pub fn load_from_file(worker_public_key: QHashOut<F>) -> Self {
        let filename = format!("{}.json", worker_public_key.to_string());
        let path = Path::new(&filename);

        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<WorkerJobTracker>(&content) {
                    Ok(tracker) => {
                        info!("Loaded job tracker from {}", filename);
                        tracker
                    }
                    Err(e) => {
                        warn!("Failed to parse job tracker from {}: {}", filename, e);
                        Self::new()
                    }
                },
                Err(e) => {
                    warn!("Failed to read job tracker file {}: {}", filename, e);
                    Self::new()
                }
            }
        } else {
            info!(
                "Job tracker file {} not found, creating new tracker",
                filename
            );
            Self::new()
        }
    }

    pub fn save_to_file(&self, worker_public_key: &str) -> anyhow::Result<()> {
        let filename = format!("{}.json", worker_public_key);
        let json_content = serde_json::to_string_pretty(self)?;

        fs::write(&filename, json_content)?;
        info!("Saved job tracker to {}", filename);
        Ok(())
    }

    pub fn add_completed_job(&mut self, job_id: QProvingJobDataID, location: JobLocation) {
        // Filter out jobs that are not supported for rewards
        if !Self::is_job_type_supported_for_rewards(job_id.circuit_type) {
            trace!(
                "Skipping job {} (type: {:?}) - not supported for rewards",
                job_id.to_hex_string(),
                job_id.circuit_type
            );
            return;
        }

        let job_hex = job_id.to_hex_string();
        let checkpoint_id = job_id.goal_id;

        match location {
            JobLocation::Coordinator => {
                self.coordinator
                    .entry(checkpoint_id)
                    .or_insert_with(Vec::new)
                    .push(job_hex);
                info!(
                    "Added coordinator job {} (type: {:?}) to checkpoint {}",
                    job_id.to_hex_string(),
                    job_id.circuit_type,
                    checkpoint_id
                );
            }
            JobLocation::Realm(realm_id) => {
                let realm_id = realm_id as u32;
                let realm_index = self
                    .realms
                    .iter()
                    .position(|r| r.id == realm_id)
                    .unwrap_or_else(|| {
                        self.realms.push(RealmJobData {
                            id: realm_id,
                            checkpoints: IndexMap::new(),
                        });
                        self.realms.len() - 1
                    });

                self.realms[realm_index]
                    .checkpoints
                    .entry(checkpoint_id)
                    .or_insert_with(Vec::new)
                    .push(job_hex);
                info!(
                    "Added realm {} job {} (type: {:?}) to checkpoint {}",
                    realm_id,
                    job_id.to_hex_string(),
                    job_id.circuit_type,
                    checkpoint_id
                );
            }
        }
    }

    pub fn get_all_jobs(&self) -> Vec<(QProvingJobDataID, String, Option<u32>)> {
        let mut all_jobs = Vec::new();

        for (checkpoint_id, job_hexs) in &self.coordinator {
            for job_hex in job_hexs {
                if let Ok(job_id) = self.parse_job_id(job_hex) {
                    all_jobs.push((job_id, "coordinator".to_string(), None));
                }
            }
        }

        for realm in &self.realms {
            for (checkpoint_id, job_hexs) in &realm.checkpoints {
                for job_hex in job_hexs {
                    if let Ok(job_id) = self.parse_job_id(job_hex) {
                        all_jobs.push((job_id, "realm".to_string(), Some(realm.id)));
                    }
                }
            }
        }

        all_jobs
    }

    fn parse_job_id(&self, hex_str: &str) -> anyhow::Result<QProvingJobDataID> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 24 {
            anyhow::bail!(
                "Invalid job ID length: expected 24 bytes, got {}",
                bytes.len()
            );
        }
        QProvingJobDataID::try_from_byte_vec(&bytes)
    }
}
