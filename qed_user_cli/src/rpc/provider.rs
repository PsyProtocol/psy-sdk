use std::{fs, sync::Arc};

use clap::{arg, Parser};
use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::store::imm::cmd_processor::QEDReadCommandProcessorSync;
use serde::{Deserialize, Serialize};

use crate::rpc::request::{
    Id, LPSResponse, QRegisterUserRPCRequest, RequestParams, ResponseResult, RpcRequest,
    RpcResponse, Version,
};

use anyhow::Ok;

use reqwest::blocking::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QSubmitEndCapRPCRequest, QTokenTransferRPCRequest,
};

use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_core::{config::network_constants::REALM_USER_TREE_HEIGHT, data::qhashout::QHashOut};

const USERS_PER_REALM_VALUE: u64 = 1u64 << (REALM_USER_TREE_HEIGHT as u64);

#[derive(Debug, Clone)]
pub struct RpcProvider {
    pub client: Arc<Client>,
    pub config: RpcConfig,
    pub current_user_id: u64,
}

#[derive(Debug)]
pub struct StorageProvider {
    pub coordinator_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
    pub realm_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
}

impl StorageProvider {
    pub fn new(config_path: &str) -> anyhow::Result<Self> {
        let config: StoreConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;

        let coordinator_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(
                &config.coordinator_store_path,
            )?);

        let realm_store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(KVQlibmdbxStore::new_read(
                &config.realm_store_path,
            )?);

        Ok(Self {
            coordinator_store,
            realm_store,
        })
    }
}

type F = GoldilocksField;
impl QEDReadCommandProcessorSync<F> for StorageProvider {
    fn resolve_batch(
        &self,
        input: &qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput,
    ) -> anyhow::Result<qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput<F>> {
        Ok(
            qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput::<F> {
                get_user_leaf: input
                    .get_user_leaf
                    .iter()
                    .map(|x| self.resolve_get_user_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_contract_leaf: input
                    .get_contract_leaf
                    .iter()
                    .map(|x| self.resolve_get_contract_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_contract_code: input
                    .get_contract_code
                    .iter()
                    .map(|x| self.resolve_get_contract_code(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_checkpoint_leaf: input
                    .get_checkpoint_leaf
                    .iter()
                    .map(|x| self.resolve_get_checkpoint_leaf(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_l2_block_state: input
                    .get_l2_block_state
                    .iter()
                    .map(|x| self.resolve_get_l2_block_state(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_merkle_proof: input
                    .get_merkle_proof
                    .iter()
                    .map(|x| self.resolve_get_merkle_proof(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                get_hash: input
                    .get_hash
                    .iter()
                    .map(|x| self.resolve_get_hash(x))
                    .collect::<anyhow::Result<Vec<_>>>()?,
            },
        )
    }

    fn resolve_get_hash(
        &self,
        input: &qed_store::store::imm::cmd::QSRHashCmd,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        if input.is_realm_cmd() {
            return self.realm_store.resolve_get_hash(input);
        }
        self.coordinator_store.resolve_get_hash(input)
    }

    fn resolve_get_merkle_proof(
        &self,
        input: &qed_store::store::imm::cmd::QSRMerkleCmd,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        if input.is_realm_cmd() {
            return self.realm_store.resolve_get_merkle_proof(input);
        }
        self.coordinator_store.resolve_get_merkle_proof(input)
    }

    fn resolve_get_user_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetUserLeafData,
    ) -> anyhow::Result<qed_data::qdata::user::QEDUserLeaf<F>> {
        self.realm_store.resolve_get_user_leaf(input)
    }

    fn resolve_get_contract_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        self.coordinator_store.resolve_get_contract_leaf(input)
    }

    fn resolve_get_contract_code(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        self.coordinator_store.resolve_get_contract_code(input)
    }

    fn resolve_get_checkpoint_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        self.realm_store.resolve_get_checkpoint_leaf(input)
    }

    fn resolve_get_l2_block_state(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetL2BlockState,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_l2_block_state(input)
    }

    fn resolve_get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        self.coordinator_store.resolve_get_latest_l2_block_state()
    }
}

impl RpcProvider {
    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_config(Default::default())
    }

    pub fn new_with_config(config: RpcConfig) -> anyhow::Result<Self> {
        assert!(config.realm_configs.len() > 0);
        Ok(Self {
            client: Arc::new(Client::new()),
            config,
            current_user_id: 0,
        })
    }
}

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

#[macro_export]
macro_rules! qed_rpc_call_back {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
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

impl QUserRpcProvider for RpcProvider {
    fn register_user<F: RichField>(&self, req: QRegisterUserRPCRequest<F>) -> anyhow::Result<()> {
        tracing::info!("register user: {:?}", req);
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs,
            RequestParams::<F>::RegisterUser(req)
        )
    }
    fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        tracing::info!("produce block");
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs,
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
        tracing::info!("deploy contract: {:?}", req);
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs,
            RequestParams::<F>::DeployContract(req)
        )
    }

    fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()> {
        // tracing::info!("submit end cap proof: {:?}", req);
        tracing::info!("submit end cap proof: {:?}", serde_json::to_string(&req));
        let rpc_url = self.get_realm_url(self.current_user_id).unwrap();
        qed_rpc_call!(self, &rpc_url, RequestParams::<F>::SubmitEndCap(req))
    }
}

impl RpcProvider {
    pub fn get_user_id<F: RichField>(&self, public_key_param: QHashOut<F>) -> anyhow::Result<u64> {
        tracing::info!("user: {:?}", public_key_param);
        let response = qed_rpc_call_back!(
            self,
            &self.config.cooridinator_configs,
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
        user_id / self.config.users_per_realm
    }

    pub fn get_realm_url(&self, user_id: u64) -> anyhow::Result<String> {
        let realm_id = self.get_realm_id(user_id);
        if realm_id >= self.config.realm_configs.len() as u64 {
            anyhow::bail!("realm id out of range");
        }
        Ok(self.config.realm_configs[realm_id as usize].clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
pub struct RpcConfig {
    #[arg(long, default_value_t = USERS_PER_REALM_VALUE, env)]
    pub users_per_realm: u64,
    #[arg(long, default_value = "http://127.0.0.1:8546", env)]
    pub realm_configs: Vec<String>,
    #[arg(long, default_value = "http://127.0.0.1:8545", env)]
    pub cooridinator_configs: String,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            users_per_realm: 1u64 << (REALM_USER_TREE_HEIGHT as u64),
            realm_configs: vec!["http://127.0.0.1:8546".into()],
            cooridinator_configs: "http://127.0.0.1:8545".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
    pub coordinator_store_path: String,
    pub realm_store_path: String,
}
