use std::{fs, sync::Arc};

use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::RichField};
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::store::imm::cmd_processor::QEDReadCommandProcessorSync;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::rpc::request::{
    Id, LPSResponse, QRegisterUserRPCRequest, RequestParams, ResponseResult, RpcRequest,
    RpcResponse, Version,
};

use anyhow::Ok;

use reqwest::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QSubmitEndCapRPCRequest, QTokenTransferRPCRequest,
};

#[derive(Clone, Debug)]
pub struct RpcProvider {
    pub client: Arc<Client>,
    pub config: RpcConfig,
    pub current_user_id: u64,
}

impl RpcProvider {
    pub fn new(config_path: &str) -> anyhow::Result<Self> {
        let config: RpcConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;
        assert!(config.realm_configs.len() > 0);
        assert!(config.cooridinator_configs.len() > 0);
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
            .send()
            .await?
            .json::<RpcResponse<()>>()
            .await?;

        if let ResponseResult::Success(s) = response.result {
            Ok(s)
        } else {
            Err(anyhow::format_err!("rpc call failed"))
        }
    }};
}

#[macro_export]
macro_rules! qed_rpc_call_back {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
        $instance
            .client
            .post($rpc_url)
            .json(&RpcRequest {
                jsonrpc: Version::V2,
                request: $rpc_params,
                id: Id::Number(1),
            })
            .send()
            .await?
            .json::<RpcResponse<LPSResponse>>()
            .await?
    }};
}

pub trait QUserRpcProvider {
    async fn register_user<F: RichField>(
        &self,
        req: QRegisterUserRPCRequest<F>,
    ) -> anyhow::Result<()>;
    async fn produce_block<F: RichField>(&self) -> anyhow::Result<()>;
    async fn add_withdrawal<F: RichField>(
        &self,
        req: QAddWithdrawalRPCRequest,
    ) -> anyhow::Result<()>;

    async fn claim_deposit<F: RichField>(&self, req: QClaimDepositRPCRequest)
        -> anyhow::Result<()>;

    async fn token_transfer<F: RichField>(
        &self,
        req: QTokenTransferRPCRequest,
    ) -> anyhow::Result<()>;

    async fn deploy_contract<F: RichField>(
        &self,
        req: QDeployContractRPCRequest<F>,
    ) -> anyhow::Result<()>;

    async fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()>;
}

impl QUserRpcProvider for RpcProvider {
    async fn register_user<F: RichField>(
        &self,
        req: QRegisterUserRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::RegisterUser(req)
        )
    }
    async fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::ProduceBlock
        )
    }
    async fn add_withdrawal<F: RichField>(
        &self,
        req: QAddWithdrawalRPCRequest,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn claim_deposit<F: RichField>(
        &self,
        req: QClaimDepositRPCRequest,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn token_transfer<F: RichField>(
        &self,
        req: QTokenTransferRPCRequest,
    ) -> anyhow::Result<()> {
        unimplemented!()
    }

    async fn deploy_contract<F: RichField>(
        &self,
        req: QDeployContractRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::DeployContract(req)
        )
    }

    async fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::SubmitEndCap(req)
        )
    }
}

type F = GoldilocksField;
impl QEDReadCommandProcessorSync<F> for RpcProvider {
    fn resolve_batch(
        &self,
        input: &qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput,
    ) -> anyhow::Result<qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput<F>> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_batch_async(input))
    }

    fn resolve_get_hash(
        &self,
        input: &qed_store::store::imm::cmd::QSRHashCmd,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_hash_async(input))
    }

    fn resolve_get_merkle_proof(
        &self,
        input: &qed_store::store::imm::cmd::QSRMerkleCmd,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_merkle_proof_async(input))
    }

    fn resolve_get_user_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetUserLeafData,
    ) -> anyhow::Result<qed_data::qdata::user::QEDUserLeaf<F>> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_user_leaf_async(input))
    }

    fn resolve_get_contract_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_contract_leaf_async(input))
    }

    fn resolve_get_contract_code(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_contract_code_async(input))
    }

    fn resolve_get_checkpoint_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_checkpoint_leaf_async(input))
    }

    fn resolve_get_l2_block_state(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetL2BlockState,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_l2_block_state_async(input))
    }

    fn resolve_get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        Runtime::new()
            .map_err(|err| anyhow::format_err!(err))?
            .block_on(self.resolve_get_latest_l2_block_state_async())
    }
}

impl RpcProvider {
    pub async fn get_user_id<F: RichField>(
        &self,
        public_key: ZKPublicKeyInfo<F>,
    ) -> anyhow::Result<u64> {
        let response = qed_rpc_call_back!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::GetUserId(public_key)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetUserId(user_id) => Ok(user_id),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    pub fn get_realm_url(&self, user_id: u64) -> anyhow::Result<String> {
        let realm_id = user_id / self.config.users_per_realm;
        if realm_id >= self.config.realm_configs.len() as u64 {
            anyhow::bail!("realm id out of range");
        }
        Ok(self.config.realm_configs[realm_id as usize].clone())
    }

    async fn resolve_batch_async(
        &self,
        input: &qed_store::store::imm::cmd_processor::QEDReadCommandBatchInput,
    ) -> anyhow::Result<qed_store::store::imm::cmd_processor::QEDReadCommandBatchOutput<F>> {
        let rpc_url = &self.config.cooridinator_configs[0];
        let response = qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::Batch(input.clone()));

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::Batch(read_command_batch) => Ok(read_command_batch),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_hash_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRHashCmd,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let user_id = input.user_id().unwrap_or(self.current_user_id);
        let rpc_url = &self.get_realm_url(user_id)?;
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetHash(input.clone()));

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_merkle_proof_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRMerkleCmd,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let user_id = input.user_id().unwrap_or(self.current_user_id);
        let rpc_url = &self.get_realm_url(user_id)?;
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetMerkleProof(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(get_merkel_proof) => Ok(get_merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_user_leaf_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetUserLeafData,
    ) -> anyhow::Result<qed_data::qdata::user::QEDUserLeaf<F>> {
        let rpc_url = &self.get_realm_url(input.user_id)?;
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserLeaf(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetUserLeaf(get_user_leaf) => Ok(get_user_leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_contract_leaf_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractLeaf(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractLeaf(get_contract_leaf) => Ok(get_contract_leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_contract_code_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        let rpc_url = &self.config.cooridinator_configs[0];
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractCode(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractCode(get_user_leaf) => Ok(get_user_leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_checkpoint_leaf_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetCheckpointLeafData,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        let rpc_url = &self.get_realm_url(self.current_user_id)?;
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointLeaf(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetCheckpointLeaf(get_chk_leaf) => Ok(get_chk_leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_l2_block_state_async(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetL2BlockState,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = &self.get_realm_url(self.current_user_id)?;
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetL2BlockState(input.clone())
        );

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetL2BlockState(get_l2_block_state) => Ok(get_l2_block_state),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn resolve_get_latest_l2_block_state_async(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = &self.get_realm_url(self.current_user_id)?;
        let response = qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetLatestL2BlockState);

        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetLatestL2BlockState(get_l2_block_state) => Ok(get_l2_block_state),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcConfig {
    pub users_per_realm: u64,
    realm_configs: Vec<String>,
    cooridinator_configs: Vec<String>,
}
