use std::sync::Arc;

use plonky2::hash::hash_types::RichField;

use crate::rpc::request::{
    Id, QRegisterUserRPCRequest, RequestParams, ResponseResult, RpcRequest, RpcResponse, Version,
};

use anyhow::Ok;

use reqwest::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QSubmitEndCapRPCRequest, QTokenTransferRPCRequest,
};

#[derive(Clone, Debug)]
pub struct RpcProvider {
    client: Arc<Client>,
    url: &'static str,
}

impl RpcProvider {
    pub fn new(url: &str) -> Self {
        Self {
            client: Arc::new(Client::new()),
            url: Box::leak(url.to_string().into_boxed_str()),
        }
    }
}

#[macro_export]
macro_rules! qed_rpc_call {
    ($instance:ident, $params:expr) => {{
        let response = $instance
            .client
            .post($instance.url)
            .json(&RpcRequest {
                jsonrpc: Version::V2,
                request: $params,
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
        qed_rpc_call!(self, RequestParams::<F>::RegisterUser(req))
    }
    async fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::ProduceBlock)
    }
    async fn add_withdrawal<F: RichField>(
        &self,
        req: QAddWithdrawalRPCRequest,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::AddWithdrawal(req))
    }

    async fn claim_deposit<F: RichField>(
        &self,
        req: QClaimDepositRPCRequest,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::ClaimDeposit(req))
    }

    async fn token_transfer<F: RichField>(
        &self,
        req: QTokenTransferRPCRequest,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::TokenTransfer(req))
    }

    async fn deploy_contract<F: RichField>(
        &self,
        req: QDeployContractRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::DeployContract(req))
    }

    async fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()> {
        qed_rpc_call!(self, RequestParams::<F>::SubmitEndCap(req))
    }
}
