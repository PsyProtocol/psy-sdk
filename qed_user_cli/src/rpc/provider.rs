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

use reqwest::blocking::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QSubmitEndCapRPCRequest, QTokenTransferRPCRequest,
};

#[derive(Debug, Clone)]
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
            .send()?
            .json::<RpcResponse<String>>()?;

        if let ResponseResult::Success(s) = response.result {
            println!("{}",s);
            Ok(())
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
            .send()?
            .json::<RpcResponse<LPSResponse>>()?
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
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::RegisterUser(req)
        )
    }
    fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        qed_rpc_call!(
            self,
            &self.config.cooridinator_configs[0],
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
            &self.config.cooridinator_configs[0],
            RequestParams::<F>::DeployContract(req)
        )
    }

    fn submit_end_cap_proof<F: RichField>(
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
        let user_id = input.user_id().unwrap_or(self.current_user_id);
        let rpc_url = match input.is_realm_cmd() {
            true => self.get_realm_url(user_id)?,
            false => self.get_cooridinator_url(user_id)?,
        };
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

    fn resolve_get_merkle_proof(
        &self,
        input: &qed_store::store::imm::cmd::QSRMerkleCmd,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let user_id = input.user_id().unwrap_or(self.current_user_id);
        let rpc_url = match input.is_realm_cmd() {
            true => self.get_realm_url(user_id)?,
            false => self.get_cooridinator_url(user_id)?,
        };
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

    fn resolve_get_user_leaf(
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

    fn resolve_get_contract_leaf(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractLeafData,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        let rpc_url = &self
            .get_cooridinator_url(self.current_user_id)
            .unwrap_or(self.config.cooridinator_configs[0].clone());
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

    fn resolve_get_contract_code(
        &self,
        input: &qed_store::store::imm::cmd::QSRCmdGetContractCodeDefinition,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        let rpc_url = &self
            .get_cooridinator_url(self.current_user_id)
            .unwrap_or(self.config.cooridinator_configs[0].clone());
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

    fn resolve_get_checkpoint_leaf(
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

    fn resolve_get_l2_block_state(
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

    fn resolve_get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = &self
            .get_cooridinator_url(self.current_user_id)
            .unwrap_or(self.config.cooridinator_configs[0].clone());
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

impl RpcProvider {
    pub fn get_user_id<F: RichField>(&self, public_key: ZKPublicKeyInfo<F>) -> anyhow::Result<u64> {
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

    pub fn get_cooridinator_url(&self, user_id: u64) -> anyhow::Result<String> {
        let cooridinator_id = user_id / self.config.users_per_realm;
        if cooridinator_id >= self.config.cooridinator_configs.len() as u64 {
            anyhow::bail!("cooridinator id out of range");
        }
        Ok(self.config.cooridinator_configs[cooridinator_id as usize].clone())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcConfig {
    pub users_per_realm: u64,
    realm_configs: Vec<String>,
    cooridinator_configs: Vec<String>,
}
