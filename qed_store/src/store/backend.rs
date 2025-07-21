use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

use super::scylla::config::ScyllaDBConfig;

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
pub struct LmdbxConfig {
    #[clap(long, env = "LMDBX_PATH", default_value = "db")]
    pub lmdbx_path: String,

    #[clap(long, env = "LMDBX_SIZE_GB", default_value = "100")]
    pub lmdbx_size_gb: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(tag = "type")]
pub enum Backend {
    #[serde(rename = "scylla")]
    Scylla(ScyllaDBConfig),

    #[serde(rename = "lmdbx")]
    Lmdbx(LmdbxConfig),
}

#[derive(Clone, Debug, Serialize, Deserialize, clap::ValueEnum, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    Scylla,
    Lmdbx,
}

impl DatabaseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseKind::Scylla => "scylla",
            DatabaseKind::Lmdbx => "lmdbx",
        }
    }
}

#[derive(Clone, Debug, Args, Serialize, Deserialize)]
pub struct BackendConfig {
    #[clap(long, env = "BACKEND_TYPE", default_value = "scylla", value_enum)]
    pub database: DatabaseKind,

    #[clap(flatten)]
    pub scylla: ScyllaDBConfig,

    #[clap(flatten)]
    pub lmdbx: LmdbxConfig,
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
                lmdbx_size_gb: 100,
            },
        }
    }
}

impl BackendConfig {
    pub fn to_backend(&self) -> Backend {
        match self.database.as_str() {
            "scylla" => Backend::Scylla(self.scylla.clone()),
            "lmdbx" => Backend::Lmdbx(self.lmdbx.clone()),
            _ => Backend::Lmdbx(self.lmdbx.clone()),
        }
    }
}
