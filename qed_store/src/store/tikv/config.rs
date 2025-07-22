use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(default)]
pub struct TiKVConfig {
    #[clap(
        long = "tikv-pd-endpoints", 
        env = "TIKV_PD_ENDPOINTS",
        help = "Comma-separated list of PD endpoints (e.g., 127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383)"
    )]
    pub pd_endpoints: String,
    
    #[clap(long = "tikv-namespace", env = "TIKV_NAMESPACE", default_value = "qed")]
    pub namespace: String,
}

impl TiKVConfig {
    pub fn get_pd_endpoints(&self) -> Vec<String> {
        if self.pd_endpoints.is_empty() {
            return vec!["127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383".to_string()];
        }
        
        self.pd_endpoints
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}

impl Default for TiKVConfig {
    fn default() -> Self {
        Self {
            pd_endpoints: "127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383".to_string(),
            namespace: "qed".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pd_endpoints() {
        let config = TiKVConfig {
            pd_endpoints: "127.0.0.1:2379".to_string(),
            namespace: "test".to_string(),
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379"]);

        let config = TiKVConfig {
            pd_endpoints: "127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383".to_string(),
            namespace: "test".to_string(),
        };
        assert_eq!(
            config.get_pd_endpoints(),
            vec!["127.0.0.1:2379", "127.0.0.1:2381", "127.0.0.1:2383"]
        );

        let config = TiKVConfig {
            pd_endpoints: "127.0.0.1:2379, 127.0.0.1:2381 , 127.0.0.1:2383".to_string(),
            namespace: "test".to_string(),
        };
        assert_eq!(
            config.get_pd_endpoints(),
            vec!["127.0.0.1:2379", "127.0.0.1:2381", "127.0.0.1:2383"]
        );

        let config = TiKVConfig {
            pd_endpoints: "".to_string(),
            namespace: "test".to_string(),
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383"]);
    }
}