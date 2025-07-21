use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

use super::scylla::config::ScyllaDBConfig;
use super::tikv::config::TiKVConfig;

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
pub struct LmdbxConfig {
    #[clap(long, env = "LMDBX_PATH", default_value = "db")]
    pub path: String,
    
    #[clap(long, env = "LMDBX_SIZE_GB", default_value = "100")]
    pub size_gb: usize,
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

#[derive(Clone, Debug, Args, Serialize, Deserialize)]
pub struct BackendConfig {
    #[clap(long, env = "BACKEND_TYPE", default_value = "scylla")]
    pub backend_type: String,
    
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
            backend_type: "scylla".to_string(),
            scylla: ScyllaDBConfig::default(),
            lmdbx: LmdbxConfig {
                path: "db".to_string(),
                size_gb: 100,
            },
            tikv: TiKVConfig::default(),
        }
    }
}

impl BackendConfig {
    pub fn to_backend(&self) -> Backend {
        match self.backend_type.as_str() {
            "scylla" => Backend::Scylla(self.scylla.clone()),
            "lmdbx" => Backend::Lmdbx(self.lmdbx.clone()),
            "tikv" => Backend::TiKV(self.tikv.clone()),
            _ => Backend::Lmdbx(self.lmdbx.clone()),
        }
    }
}