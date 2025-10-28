use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(default)]
pub struct TiKVConfig {
    #[clap(
        long = "tikv-pd-endpoints",
        env = "TIKV_PD_ENDPOINTS",
        default_value = "127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383",
        help = "Comma-separated list of PD endpoints (e.g., 127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383)"
    )]
    pub pd_endpoints: String,

    #[clap(long = "tikv-namespace", env = "TIKV_NAMESPACE", default_value = "psy")]
    pub namespace: String,
    #[clap(long = "tikv-timeout", env = "TIKV_TIMEOUT", default_value = "2")]
    pub timeout: u64,
    #[clap(
        long = "tikv-grpc-max-decoding-message-size",
        env = "TIKV_GRPC_MAX_DECODING_MESSAGE_SIZE",
        default_value = "20971520"
    )] // 20MB
    pub grpc_max_decoding_message_size: usize,
    #[clap(
        long = "tikv-enable-grpc-gzip-compression",
        env = "TIKV_ENABLE_GRPC_GZIP_COMPRESSION",
        default_value = "true"
    )]
    pub enable_grpc_gzip_compression: bool,
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
            namespace: "psy".to_string(),
            timeout: 10,
            grpc_max_decoding_message_size: 20 * 1024 * 1024, // 20MB => 20971520
            enable_grpc_gzip_compression: true,
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
            ..Default::default()
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379"]);

        let config = TiKVConfig {
            pd_endpoints: "127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383".to_string(),
            ..Default::default()
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379", "127.0.0.1:2381", "127.0.0.1:2383"]);

        let config = TiKVConfig {
            pd_endpoints: "127.0.0.1:2379, 127.0.0.1:2381 , 127.0.0.1:2383".to_string(),
            ..Default::default()
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379", "127.0.0.1:2381", "127.0.0.1:2383"]);

        let config = TiKVConfig {
            pd_endpoints: "".to_string(),
            ..Default::default()
        };
        assert_eq!(config.get_pd_endpoints(), vec!["127.0.0.1:2379,127.0.0.1:2381,127.0.0.1:2383"]);
    }
}
