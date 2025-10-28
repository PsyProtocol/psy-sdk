use clap::{Args, Parser};
use serde::{Deserialize, Serialize};
use super::scylla::config::ScyllaDBConfig;
use super::tikv::config::TiKVConfig;

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
pub struct LmdbxConfig {
    #[clap(long, env = "LMDBX_PATH", default_value = "db")]
    pub lmdbx_path: String,

    #[clap(long, env = "LMDBX_MMAP_SIZE_GB", default_value = "100")]
    pub lmdbx_mmap_size_gb: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(tag = "type")]
pub enum Backend {
    #[serde(rename = "scylla")]
    Scylla(ScyllaDBConfig),

    #[serde(rename = "lmdbx")]
    Lmdbx(LmdbxConfig),
    
    #[serde(rename = "tikv")]
    TiKV(TiKVConfig),
}

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Scylla,
    Lmdbx,
    Tikv,
}

#[derive(Clone, Debug, Args, Serialize, Deserialize)]
pub struct BackendConfig {
    #[clap(long, env = "DATABASE_KIND", default_value = "scylla", value_enum)]
    pub database: DatabaseKind,

    #[clap(flatten)]
    pub scylla: ScyllaDBConfig,

    #[clap(flatten)]
    pub lmdbx: LmdbxConfig,
    
    #[clap(flatten)]
    pub tikv: TiKVConfig,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Scylla(ScyllaDBConfig::default())
    }
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            database: DatabaseKind::Scylla,
            scylla: ScyllaDBConfig::default(),
            lmdbx: LmdbxConfig {
                lmdbx_path: "db".to_string(),
                lmdbx_mmap_size_gb: 100,
            },
            tikv: TiKVConfig::default(),
        }
    }
}

impl BackendConfig {
    pub fn to_backend(&self) -> Backend {
        match self.database {
            DatabaseKind::Scylla => Backend::Scylla(self.scylla.clone()),
            DatabaseKind::Lmdbx => Backend::Lmdbx(self.lmdbx.clone()),
            DatabaseKind::Tikv => Backend::TiKV(self.tikv.clone()),
        }
    }
}
