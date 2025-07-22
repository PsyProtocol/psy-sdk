use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
pub struct TiKVConfig {
    #[clap(long = "tikv-pd-endpoints", env = "TIKV_PD_ENDPOINTS", default_value = "127.0.0.1:2379")]
    pub pd_endpoints: Vec<String>,
    
    #[clap(long = "tikv-namespace", env = "TIKV_NAMESPACE", default_value = "qed")]
    pub namespace: String,
    
    #[clap(long = "tikv-timeout-ms", env = "TIKV_TIMEOUT_MS", default_value_t = 30000)]
    pub timeout_ms: u64,
}

impl Default for TiKVConfig {
    fn default() -> Self {
        Self {
            pd_endpoints: vec!["127.0.0.1:2379".to_string()],
            namespace: "qed".to_string(),
            timeout_ms: 30000,
        }
    }
}

impl TiKVConfig {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}
