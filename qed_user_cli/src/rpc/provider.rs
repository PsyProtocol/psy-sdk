use std::{collections::HashMap, fs, sync::Arc};
use plonky2::{hash::hash_types::RichField};
use serde::{Deserialize, Serialize};

use crate::rpc::request::{
    Id, QRegisterUserRPCRequest, RequestParams, ResponseResult, RpcRequest,
    RpcResponse, Version,
};

use anyhow::Ok;
use rand::Rng;

#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;

#[cfg(target_arch = "wasm32")]
use reqwest::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QSubmitEndCapRPCRequest, QTokenTransferRPCRequest,
};

use qed_core::{config::network_constants::REALM_USER_TREE_HEIGHT, data::qhashout::QHashOut};

#[derive(Debug, Clone)]
pub struct RpcProvider {
    pub client: Arc<Client>,
    pub realm_configs: HashMap<u64, Vec<String>>,
    pub coordinator_configs: HashMap<u64, Vec<String>>,
    pub users_per_realm: u64,
    pub current_user_id: u64,
}


impl RpcProvider {
    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_config(&Default::default())
    }

    pub fn new_with_config_path(config: &str) -> anyhow::Result<Self> {
        let config: RpcConfig = serde_json::from_str(&fs::read_to_string(config)?)?;
        Self::new_with_config(&config)
    }

    pub fn new_with_config(config: &RpcConfig) -> anyhow::Result<Self> {
        tracing::info!(
            "start rpc provider with config: {}",
            serde_json::to_string_pretty(&config)?
        );
        assert!(config.realm_configs.len() > 0);
        assert!(config.coordinator_configs.len() > 0);
        let mut realm_configs = HashMap::new();
        let mut coordinator_configs = HashMap::new();

        config.realm_configs.iter().for_each(|realm_config| {
            assert!(realm_config.rpc_url.len() > 0);
            realm_configs.insert(realm_config.id, realm_config.rpc_url.clone());
        });
        config
            .coordinator_configs
            .iter()
            .for_each(|coordinator_config| {
                assert!(coordinator_config.rpc_url.len() > 0);
                coordinator_configs
                    .insert(coordinator_config.id, coordinator_config.rpc_url.clone());
            });

        Ok(Self {
            client: Arc::new(Client::new()),
            realm_configs,
            coordinator_configs,
            users_per_realm: config.users_per_realm,
            current_user_id: 0,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! qed_rpc_call {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
        let response = $instance
            .client
            .post($rpc_url)
            .json(&RpcRequest {
                jsonrpc: Version::V2,
                request: $rpc_params,
                id: Id::Number(1),
            })
            .send()?
            .json::<RpcResponse<String>>()?;

        match response.result {
            ResponseResult::Success(s) => {
                tracing::info!("{:?}", s);
                Ok(())
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("qed rpc call failed `{:?}`", e)),
        }
    }};
}

#[cfg(target_arch = "wasm32")]
macro_rules! qed_rpc_call {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
        use pollster::FutureExt as _;
        (async move {
            let response = $instance
                .client
                .post($rpc_url)
                .json(&RpcRequest {
                    jsonrpc: Version::V2,
                    request: $rpc_params,
                    id: Id::Number(1),
                })
                .send()
                .await?
                .json::<RpcResponse<String>>()
                .await?;

            match response.result {
                ResponseResult::Success(s) => {
                    tracing::info!("{:?}", s);
                    Ok(())
                }
                ResponseResult::Error(e) => Err(anyhow::format_err!("qed rpc call failed `{:?}`", e)),
            }
        }).block_on()
    }};
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! qed_rpc_call_back {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
        tracing::info!("qed rpc call: {}", $rpc_url);
        $instance
            .client
            .post($rpc_url)
            .json(&RpcRequest {
                jsonrpc: Version::V2,
                request: $rpc_params,
                id: Id::Number(1),
            })
            .send()?
            .json::<RpcResponse<$ret_ty>>()?
    }};
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! qed_rpc_call_back {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
        use pollster::FutureExt as _;
        (async move {
            tracing::info!("qed rpc call: {}", $rpc_url);
            $instance.client
                .post($rpc_url)
                .json(&RpcRequest {
                    jsonrpc: Version::V2,
                    request: $rpc_params,
                    id: Id::Number(1),
                })
                .send()
                .await?
                .json::<RpcResponse<$ret_ty>>()
                .await
        }).block_on()?
    }};
}

// #[cfg(not(target_arch = "wasm32"))]
pub trait QUserRpcProvider {
    fn register_user<F: RichField>(&self, req: QRegisterUserRPCRequest<F>) -> anyhow::Result<()>;
    fn produce_block<F: RichField>(&self) -> anyhow::Result<()>;
    fn add_withdrawal<F: RichField>(&self, req: QAddWithdrawalRPCRequest) -> anyhow::Result<()>;

    fn claim_deposit<F: RichField>(&self, req: QClaimDepositRPCRequest) -> anyhow::Result<()>;

    fn token_transfer<F: RichField>(&self, req: QTokenTransferRPCRequest) -> anyhow::Result<()>;

    fn deploy_contract<F: RichField>(
        &self,
        req: QDeployContractRPCRequest<F>,
    ) -> anyhow::Result<()>;

    fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()>;
}

// #[cfg(not(target_arch = "wasm32"))]
impl QUserRpcProvider for RpcProvider {
    fn register_user<F: RichField>(&self, req: QRegisterUserRPCRequest<F>) -> anyhow::Result<()> {
        tracing::info!("register user: {:?}", req);
        qed_rpc_call!(
            self,
            self.get_coordinator_url()?,
            RequestParams::<F>::RegisterUser(req)
        )
    }
    fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        tracing::info!("produce block");
        qed_rpc_call!(
            self,
            self.get_coordinator_url()?,
            RequestParams::<F>::ProduceBlock
        )
    }
    fn add_withdrawal<F: RichField>(&self, req: QAddWithdrawalRPCRequest) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn claim_deposit<F: RichField>(&self, req: QClaimDepositRPCRequest) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn token_transfer<F: RichField>(&self, req: QTokenTransferRPCRequest) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn deploy_contract<F: RichField>(
        &self,
        req: QDeployContractRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            self.get_coordinator_url()?,
            RequestParams::<F>::DeployContract(req)
        )
    }

    fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()> {
        // tracing::info!(
        //     "submit end cap proof: {}",
        //     serde_json::to_string_pretty(&req).unwrap()
        // );
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        qed_rpc_call!(self, rpc_url, RequestParams::<F>::SubmitEndCap(req))
    }
}

impl RpcProvider {
    pub fn get_user_id<F: RichField>(&self, public_key_param: QHashOut<F>) -> anyhow::Result<u64> {
        tracing::info!("user: {:?}", public_key_param);
        let rpc_url = self.get_coordinator_url()?;  
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserId(public_key_param),
            u64
        );
        match response.result {
            ResponseResult::Success(user_id) => {
                tracing::info!("get user id: {:?}", user_id);
                Ok(user_id)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    pub const fn get_realm_id(&self, user_id: u64) -> u64 {
        user_id / self.users_per_realm
    }

    pub fn get_realm_url(&self, user_id: u64) -> anyhow::Result<&String> {
        let realm_id = self.get_realm_id(user_id);

        let realm_urls = self
            .realm_configs
            .get(&realm_id)
            .ok_or(anyhow::format_err!(
                "realm id `{}` not found, please check the config",
                realm_id
            ))?;
        let random_index = rand::thread_rng().gen_range(0..realm_urls.len());

        Ok(&realm_urls[random_index])
    }

    pub fn get_coordinator_url(&self) -> anyhow::Result<&String> {
        let coordinator_urls = self.coordinator_configs.get(&0).ok_or(anyhow::format_err!(
            "coordinator id `{}` not found, please check the config",
            0
        ))?;
        let random_index = rand::thread_rng().gen_range(0..coordinator_urls.len());
        Ok(&coordinator_urls[random_index])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcConfig {
    pub users_per_realm: u64,
    pub realm_configs: Vec<RealmRpcConfig>,
    pub coordinator_configs: Vec<CoordinatorRpcConfig>,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            users_per_realm: 1u64 << (REALM_USER_TREE_HEIGHT as u64),
            realm_configs: vec![
                RealmRpcConfig {
                    id: 0,
                    rpc_url: vec!["http://127.0.0.1:8546".into()],
                },
                RealmRpcConfig {
                    id: 2048,
                    rpc_url: vec!["http://127.0.0.1:8547".into()],
                },
            ],
            coordinator_configs: vec![CoordinatorRpcConfig {
                id: 0,
                rpc_url: vec!["http://127.0.0.1:8545".into()],
            }],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RealmRpcConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoordinatorRpcConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
    pub coordinator_store_path: String,
    pub realm_store_path: String,
}
