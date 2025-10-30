use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64}, hash::hash_types::{HashOut, RichField}, plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{circuits::zk_signature3::core::QEDBasicZKSignatureInnerCircuit, treeprover::qrecursion::standard::manager::portable::circuits::{
    PortableQTreeRecursionCircuitsDataTrait, PortableQTreeRecursionCircuitsProveTrait,
    PortableQTreeRecursionCircuitsTrait,
}};
use qed_core::{job::id::{QProvingJobDataID, VariableHeightRewardMerkleProof}, traits::to_qfelts::ToQFelts};
use qed_crypto::{
    common::witnesses::qrecursion::proof_data::{
        AggProofRecord, SimpleQTreeRecursionManagerInclusionProofs,
    },
    signature::secp256k1::core::QEDCompressedSecp256K1Signature,
};
use qed_crypto::{
    common::witnesses::qrecursion::{
        header::QRecursionAggStandardHeader, proof_data::QStandardBinaryTreeCircuitType,
    },
    hash::{
        merkle::core::{DeltaMerkleProofCore, MerkleProofCore},
        traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable},
    },
};
use qed_data::{
    config::store_config::{QEDFelt, QEDHasher}, qdata::{checkpoint::QEDL2BlockState, contract::ContractCodeDefinition, user}, traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QTreeDataStoreReaderSync}, ups::{
        start_step::UPSStartStepInput,
        ups_cfc_standard_step::{
            UPSCFCDeferredTransactionCircuitInput, UPSCFCStandardTransactionCircuitInput,
        },
        ups_end_cap::UPSEndCapFromProofTreeGadgetInput,
    }
};
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qed_store::controllers::local::session_info::SessionCircuitInfoStore;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, marker::PhantomData, sync::Arc};

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub enum JobLocation {
    Realm(u64),
    Coordinator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub job_id: QProvingJobDataID,
    pub location: JobLocation,
}

use crate::{local::request::{
    QGetContractMethodCommonDataRPCRequest, QGetMethodIdRPCRequest, QGetTxStatusRPCRequest, QL2BlockStateRPCRequest, QLatestL2BlockStateRPCRequest, QLeftAggRightLeafRpcRequestV2, QLeftLeafRightAggRpcRequestV2, QProveContractCallRPCRequest, QProveUpsStartRPCRequest, QRegisterCircuitsRPCRequest, QRegisterSoftwareDefinedCircuitRPCRequest, QSecpSignatureProofRPCRequest, QSignatureMinifierProofRPCRequest, QSignatureProofRPCRequest, QSingleLeafRpcRequestV2, QSoftwareDefinedSignatureProofRPCRequest, QTwoAggRpcRequsetV2, QTwoLeafRpcRequestV2, QUpsCfcDeferredTxRPCRequest, QUpsCfcStandardTxRPCRequest, QUpsEndCapRPCRequestV2, QUserSubTreeMerkleProofRPCRequest, RequestParamsV2
}, session::TxStatus};
use crate::wallet::software_defined_circuit::{
    SoftwareDefinedSignatureInput, SoftwareDefinedSignatureWitnessInput,
};

use super::request::{
    Id, QRegisterUserRPCRequest, RequestParams, ResponseResult, RpcRequest, RpcResponse, Version,
};
use serde_json;

use std::result::Result::Ok;
// #[cfg(not(target_arch = "wasm32"))]
use rand::Rng;

// #[cfg(not(target_arch = "wasm32"))]
// use reqwest::blocking::Client;

// #[cfg(target_arch = "wasm32")]
use reqwest::Client;

use super::request::{
    QAddWithdrawalRPCRequest, QClaimDepositRPCRequest, QDeployContractRPCRequest,
    QGetUserIdRPCRequest, QSubmitEndCapRPCRequest, QSubmitGutaRPCRequest, QTokenTransferRPCRequest,
};

use qed_core::{
    config::network_constants::REALM_USER_TREE_HEIGHT,
    data::{alt::AltVerifierOnlyCircuitData, qhashout::QHashOut},
    ups::circuits::LocalCircuitType,
};

#[derive(Debug, Clone)]
pub struct RpcProvider {
    pub client: Arc<Client>,
    pub realm_configs: HashMap<u64, Vec<String>>,
    pub coordinator_configs: HashMap<u64, Vec<String>>,
    pub users_per_realm: u64,
    pub current_user_id: u64,
}

impl RpcProvider {
    pub fn new_with_config_path(config_path: &str) -> anyhow::Result<Self> {
        let config_str = fs::read_to_string(config_path)?;
        let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
        let config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
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

// #[cfg(any(not(target_arch = "wasm32"), feature = "is_sync"))]
// #[macro_export]
// macro_rules! qed_rpc_call {
//     ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
//         let response = $instance
//             .client
//             .post($rpc_url)
//             .json(&RpcRequest {
//                 jsonrpc: Version::V2,
//                 request: $rpc_params,
//                 id: Id::Number(1),
//             })
//             .send()?
//             .json::<RpcResponse<String>>()?;

//         match response.result {
//             ResponseResult::Success(s) => {
//                 tracing::info!("{:?}", s);
//                 Ok(())
//             }
//             ResponseResult::Error(e) => Err(anyhow::format_err!("qed rpc call failed `{:?}`", e)),
//         }
//     }};
// }

// #[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! qed_rpc_call {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
        async move {
            let request = RpcRequest {
                jsonrpc: Version::V2,
                request: $rpc_params,
                id: Id::Number(1),
            };
            let response = $instance
                .client
                .post($rpc_url)
                .json(&request)
                .send()
                .await?;
            let json_response: RpcResponse<String> = response.json().await?;
            match json_response.result {
                ResponseResult::Success(s) => {
                    tracing::info!("{:?}", s);
                    Ok(())
                }
                ResponseResult::Error(e) => {
                    Err(anyhow::format_err!("qed rpc call failed `{:?}`", e))
                }
            }
        }
        .await
    }};
}

// #[cfg(target_arch = "wasm32")]
// #[macro_export]
// macro_rules! qed_rpc_call {
//     ($instance:ident, $rpc_url:expr, $rpc_params:expr) => {{
//         use std::sync::mpsc;
//         use std::sync::mpsc::{Sender, Receiver};
//         use wasm_bindgen_futures::spawn_local;
//         use anyhow::Result;

//         let (tx, rx) = mpsc::channel::<Result<()>>();

//         let client = $instance.client.clone();
//         let rpc_url = $rpc_url.to_string();
//         let rpc_params = $rpc_params;

//         spawn_local(async move {
//             let result: Result<()> = {
//                 tracing::info!("qed rpc call (wasm): {}", rpc_url);
//                 let response = client
//                     .post(&rpc_url)
//                     .timeout(std::time::Duration::from_secs(360))
//                     .json(&RpcRequest {
//                         jsonrpc: Version::V2,
//                         request: rpc_params,
//                         id: Id::Number(1),
//                     })
//                     .send()
//                     .await
//                     .map_err(|e| anyhow::anyhow!("qed rpc call failed: {}", e));

//                 match response {
//                     Ok(response) => {
//                         match response.json::<RpcResponse<String>>().await {
//                             Ok(json_response) => Ok(()),
//                             Err(e) => Err(anyhow::anyhow!("parse reponse failed: {}", e)),
//                         }
//                     }
//                     Err(e) => Err(anyhow::anyhow!("qed rpc call failed: {}", e)),
//                 }
//             };

//             let _ = tx.send(result);
//         });

//         rx.recv().map_err(|e| anyhow::anyhow!("channel reception failed: {}", e))?
//     }};
// }

// #[cfg(any(not(target_arch = "wasm32"), feature = "is_sync"))]
// #[macro_export]
// macro_rules! qed_rpc_call_back {
//     ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
//         tracing::info!("qed rpc call: {}", $rpc_url);
//         let request = RpcRequest {
//             jsonrpc: Version::V2,
//             request: $rpc_params,
//             id: Id::Number(1),
//         };
//         $instance
//             .client
//             .post($rpc_url)
//             .timeout(std::time::Duration::from_millis(1000))
//             .json(&request)
//             .send()?
//             .json::<RpcResponse<$ret_ty>>()?
//     }};
// }

// #[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! qed_rpc_call_back {
    ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
        async move {
            tracing::info!("qed rpc call: {}", $rpc_url);
            $instance
                .client
                .post($rpc_url)
                .timeout(std::time::Duration::from_secs(360))
                .json(&RpcRequest {
                    jsonrpc: Version::V2,
                    request: $rpc_params,
                    id: Id::Number(1),
                })
                .send()
                .await?
                .json::<RpcResponse<$ret_ty>>()
                .await
        }
        .await?
    }};
}

// #[cfg(target_arch = "wasm32")]
// #[macro_export]
// macro_rules! qed_rpc_call_back {
//     ($instance:ident, $rpc_url:expr, $rpc_params:expr, $ret_ty: ty) => {{
//         use std::sync::mpsc;
//         use std::sync::mpsc::{Sender, Receiver};
//         use wasm_bindgen_futures::spawn_local;
//         use anyhow::Result;

//         let (tx, rx) = mpsc::channel::<Result<RpcResponse<$ret_ty>>>();

//         let client = $instance.client.clone();
//         let rpc_url = $rpc_url.to_string();
//         let rpc_params = $rpc_params;

//         spawn_local(async move {
//             let result: Result<RpcResponse<$ret_ty>> = {
//                 tracing::info!("qed rpc call (wasm): {}", rpc_url);
//                 let response = client
//                     .post(&rpc_url)
//                     .timeout(std::time::Duration::from_secs(360))
//                     .json(&RpcRequest {
//                         jsonrpc: Version::V2,
//                         request: rpc_params,
//                         id: Id::Number(1),
//                     })
//                     .send()
//                     .await
//                     .map_err(|e| anyhow::anyhow!("qed rpc call failed: {}", e));

//                 match response {
//                     Ok(response) => {
//                         match response.json::<RpcResponse<$ret_ty>>().await {
//                             Ok(json_response) => Ok(json_response),
//                             Err(e) => Err(anyhow::anyhow!("parse reponse failed: {}", e)),
//                         }
//                     }
//                     Err(e) => Err(anyhow::anyhow!("qed rpc call failed: {}", e)),
//                 }
//             };

//             let _ = tx.send(result);
//         });

//         let rpc_result = rx.recv()
//             .map_err(|e| anyhow::anyhow!("通道接收失败: {}", e))?;

//         rpc_result
//             .map_err(|e| anyhow::anyhow!("qed rpc call failed: {}", e))?
//     }};
// }

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
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

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl QUserRpcProvider for RpcProvider {
    async fn register_user<F: RichField>(
        &self,
        req: QRegisterUserRPCRequest<F>,
    ) -> anyhow::Result<()> {
        tracing::info!("register user: {:?}", req);
        let url = self.get_coordinator_url()?;
        qed_rpc_call!(self, url, RequestParams::<F>::RegisterUser(req))
    }
    async fn produce_block<F: RichField>(&self) -> anyhow::Result<()> {
        tracing::info!("produce block");
        let url = self.get_coordinator_url()?;
        qed_rpc_call!(self, url, RequestParams::<F>::ProduceBlock)
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
        let url = self.get_coordinator_url()?;
        qed_rpc_call!(self, url, RequestParams::<F>::DeployContract(req))
    }

    async fn submit_end_cap_proof<F: RichField>(
        &self,
        req: QSubmitEndCapRPCRequest<F>,
    ) -> anyhow::Result<()> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        qed_rpc_call!(self, rpc_url, RequestParams::<F>::SubmitEndCap(req))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl RpcProvider {
    pub async fn get_user_id<F: RichField>(&self, public_key: QHashOut<F>) -> anyhow::Result<u64> {
        tracing::info!("user: {}", public_key);
        let url = self.get_coordinator_url()?;
        let response = qed_rpc_call_back!(
            self,
            url,
            RequestParams::<F>::GetUserId(QGetUserIdRPCRequest { public_key }),
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

    pub async fn get_realm_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        tracing::info!("Fetching latest realm L2 block state");
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QLatestL2BlockStateRPCRequest {};
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<GoldilocksField>::GetLatestL2BlockState(input),
            QEDL2BlockState
        );
        match response.result {
            ResponseResult::Success(block_state) => {
                tracing::debug!(
                    block_state = %serde_json::to_string_pretty(&block_state).unwrap(),
                    "Successfully fetched L2 block state"
                );
                Ok(block_state)
            }
            ResponseResult::Error(e) => {
                tracing::error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_realm_latest_l2_block_state rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    pub async fn get_realm_l2_block_state(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        tracing::info!("Fetching realm L2 block state at checkpoint {}", checkpoint_id);
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QL2BlockStateRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<GoldilocksField>::GetL2BlockState(input),
            QEDL2BlockState
        );
        match response.result {
            ResponseResult::Success(block_state) => {
                tracing::debug!(
                    block_state = %serde_json::to_string_pretty(&block_state).unwrap(),
                    "Successfully fetched L2 block state"
                );
                Ok(block_state)
            }
            ResponseResult::Error(e) => {
                tracing::error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_realm_l2_block_state rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    pub async fn get_user_sub_tree_merkle_proof_inner(
        &self,
        rpc_url: &str,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<GoldilocksField>>> {
        let input = QUserSubTreeMerkleProofRPCRequest {
            checkpoint_id,
            root_level,
            leaf_level,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<GoldilocksField>::GetUserSubTreeMerkleProof(input),
            MerkleProofCore<QHashOut<GoldilocksField>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                tracing::debug!(
                    checkpoint_id = checkpoint_id,
                    root_level = root_level,
                    leaf_level = leaf_level,
                    leaf_index = leaf_index,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                tracing::error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_sub_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    pub async fn get_job_proofs(
        &self,
        job_infos: Vec<JobInfo>,
    ) -> anyhow::Result<Vec<(QProvingJobDataID, VariableHeightRewardMerkleProof)>> {

        let mut jobs_by_checkpoint: HashMap<u64, HashMap<JobLocation, Vec<JobInfo>>> = HashMap::new();

        for job_info in job_infos {
            let checkpoint_id = job_info.job_id.goal_id;
            jobs_by_checkpoint
                .entry(checkpoint_id)
                .or_insert_with(HashMap::new)
                .entry(job_info.location.clone())
                .or_insert_with(Vec::new)
                .push(job_info);
        }

        let mut all_results = Vec::new();

        for (checkpoint_id, jobs_by_location) in jobs_by_checkpoint {
            for (location, jobs) in jobs_by_location {
                if jobs.is_empty() {
                    continue;
                }

                let url = match &location {
                    JobLocation::Coordinator => self.get_coordinator_url()?,
                    JobLocation::Realm(realm_id) => {
                        let realm_urls = self
                            .realm_configs
                            .get(realm_id)
                            .ok_or(anyhow::format_err!("Realm {} not configured", realm_id))?;
                        &realm_urls[0]
                    }
                };

                let output_job_ids: Vec<_> = jobs.iter().map(|j| j.job_id.get_output_id()).collect();
                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "qed_generate_batch_variable_height_reward_proofs",
                    "params": [checkpoint_id, output_job_ids],
                    "id": 1
                });

                let response = self
                    .client
                    .post(url)
                    .json(&request)
                    .send()
                    .await?
                    .json::<serde_json::Value>()
                    .await?;

                if let Some(error) = response.get("error") {
                    return Err(anyhow::format_err!("RPC error: {:?}", error));
                }

                let result = response
                    .get("result")
                    .ok_or(anyhow::format_err!("Missing result in RPC response"))?;

                let proofs: Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)> = serde_json::from_value(result.clone())?;

                for (proof, job_id) in proofs {
                    all_results.push((job_id, proof));
                }
            }
        }

        Ok(all_results)
    }

    pub async fn get_claim_amount(&self, checkpoint_id: u64, user_id: u64, claim_user_id: u64) -> anyhow::Result<u64> {
        let contract_id = 0;
        let height = 32;
        let sender_total_sent_index = 3 + user_id * 2;
        let sender_total_sent_slot = sender_total_sent_index / 4;
        let sender_total_sent_slot_index = sender_total_sent_index % 4;
        let amount_claimed_index = 3 + claim_user_id * 2 + 1;
        let amount_claimed_slot = amount_claimed_index / 4;
        let amount_claimed_slot_index = amount_claimed_index % 4;
        let user_total_sent = self
            .get_user_contract_state_tree_leaf_hash(checkpoint_id, claim_user_id, contract_id, height, sender_total_sent_slot)
            .await?.to_qfelts()[(sender_total_sent_slot_index) as usize].to_canonical_u64();
        let amount_claimed = self
            .get_user_contract_state_tree_leaf_hash(checkpoint_id, user_id, contract_id, height, amount_claimed_slot)
            .await?.to_qfelts()[(amount_claimed_slot_index) as usize].to_canonical_u64();

        if amount_claimed > user_total_sent {
            return Err(anyhow::format_err!("amount claimed {} is greater than user total sent {}", amount_claimed, user_total_sent));
        }

        Ok(user_total_sent - amount_claimed)
    }

    pub async fn check_tx_is_confirmed(&self, checkpoint_id: u64, user_id: u64, tx_hash: QHashOut<GoldilocksField>) -> anyhow::Result<bool> {
        let user_leaf_data = self.get_user_leaf_data(checkpoint_id, user_id).await?;
        Ok(user_leaf_data.qfhash::<QEDHasher>() == tx_hash)
    }

    pub async fn get_tx_status(
        &self,
        user_id: u64,
        nonce: u64,
    ) -> anyhow::Result<TxStatus> {
        tracing::info!(
            "Fetching tx status user_id: {}, nonce: {}",
            user_id, nonce
        );
        let rpc_url = self.get_realm_url(user_id)?;

        let input = QGetTxStatusRPCRequest {
            user_id,
            nonce,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<QEDFelt>::GetTxStatus(input),
            TxStatus
        );
        match response.result {
            ResponseResult::Success(status) => {
                tracing::info!(
                    "Successfully fetched tx status user_id: {}, nonce: {}, status: {:?}",
                    user_id,
                    nonce,
                    status
                );
                Ok(status)
            }
            ResponseResult::Error(e) => {
                tracing::error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_tx_status rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    pub const fn get_realm_id(&self, user_id: u64) -> u64 {
        user_id / self.users_per_realm
    }

    pub fn get_realm_url(&self, user_id: u64) -> anyhow::Result<&String> {
        let realm_id = self.get_realm_id(user_id);
        tracing::info!("get realm url for user id {}, realm id {}", user_id, realm_id);

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
    pub global_user_tree_height: u8,
    pub realm_user_tree_height: u8,
    pub realm_configs: Vec<RealmRpcConfig>,
    pub coordinator_configs: Vec<CoordinatorRpcConfig>,
    pub prove_proxy_url: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub users_per_realm: u64,
    pub global_user_tree_height: u8,
    pub realm_user_tree_height: u8,
    pub realm_configs: Vec<RealmConfig>,
    pub coordinator_configs: Vec<CoordinatorConfig>,
    pub prover_url: Option<String>,
    pub prove_proxy_url: Vec<String>,
    pub native_currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

// type C = PoseidonGoldilocksConfig;
// const D: usize = 2;
// type F = <C as GenericConfig<D>>::F;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait UPSCircuitManagerTrait<C: GenericConfig<D>, const D: usize>: PortableQTreeRecursionCircuitsTrait<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>);
    async fn prove_ups_start(
        &self,
        input: &UPSStartStepInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: &ContractCodeDefinition,
    ) -> anyhow::Result<()>;

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64>;

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)>;

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_zk_sign(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_secp_sign(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn register_software_defined_circuit(
        &self,
        input: SoftwareDefinedSignatureInput,
    ) -> anyhow::Result<QHashOut<C::F>>;

    async fn prove_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: SoftwareDefinedSignatureWitnessInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    // async fn software_defined_sign(
    //     &self,
    //     private_key: QHashOut<C::F>,
    //     sig_hash: QHashOut<C::F>,
    // ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    // async fn finalize_tree(&self) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_start_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_cfc_standard_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_cfc_deferred_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_end_cap_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize, T> UPSCircuitManagerTrait<C, D> for &T
where
    T: UPSCircuitManagerTrait<C, D> + Sync,
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        (**self).register_info(info_store).await
    }

    async fn prove_ups_start(
        &self,
        input: &UPSStartStepInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_start(input).await
    }

    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        (** self).register_contract_circuits(contract_id, contract_code).await
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64> {
        (**self).get_method_id(contract_id, method_name).await
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        (** self).get_contract_method_common_data(contract_id, method_id).await
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_contract_call(contract_id, method_id, input).await
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (** self).prove_ups_cfc_standard_tx(input).await
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_cfc_deferred_tx(input).await
    }

    async fn prove_zk_sign(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (** self).prove_zk_sign(private_key, sig_hash).await
    }

    async fn prove_secp_sign(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_secp_sign(signature).await
    }

    async fn register_software_defined_circuit(
        &self,
        input: SoftwareDefinedSignatureInput,
    ) -> anyhow::Result<QHashOut<C::F>> {
        (** self).register_software_defined_circuit(input).await
    }

    async fn prove_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: SoftwareDefinedSignatureWitnessInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_software_defined_sign(fingerprint, private_key, input, sig_hash).await
    }

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (** self).prove_ups_end_cap(circuit_info, end_cap_from_proof_tree_input, agg_proof_record).await
    }

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_start_circuit_fingerprint().await
    }

    async fn ups_start_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (** self).ups_start_circuit_verifier_config().await
    }

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_standard_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_standard_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (** self).ups_cfc_standard_tx_circuit_verifier_config().await
    }

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_deferred_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_deferred_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (** self).ups_cfc_deferred_tx_circuit_verifier_config().await
    }

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_end_cap_circuit_fingerprint().await
    }

    async fn ups_end_cap_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (** self).ups_end_cap_circuit_verifier_config().await
    }

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_circuit_whitelist_root().await
    }

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (** self).zk_circuit_fingerprint().await
    }

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).zk_circuit_verifier_config().await
    }

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (** self).secp_circuit_fingerprint().await
    }

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).secp_circuit_verifier_config().await
    }
}

#[derive(Clone, Debug)]
pub struct ProveProxyRpcProvider<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub client: Arc<Client>,
    pub proof_proxy_url: String,
    pub common_circuits_data: LocalCommonCircuitsData<C::F>,
    pub zk_sign_inner_circuit: QEDBasicZKSignatureInnerCircuit<C, D>,
    pub _marker: PhantomData<C>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QCommonCircuitData<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub verifier_config: AltVerifierOnlyCircuitData<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct LocalCommonCircuitsData<F: RichField> {
    pub ups_start: QCommonCircuitData<F>,
    pub ups_cfc_standard_tx: QCommonCircuitData<F>,
    pub ups_cfc_deferred_tx: QCommonCircuitData<F>,
    pub ups_end_cap: QCommonCircuitData<F>,
    pub zk_circuit: QCommonCircuitData<F>,
    pub secp_circuit: QCommonCircuitData<F>,

    pub ups_circuit_whitelist_root: QHashOut<F>,
    pub ups_start_whitelist_proof: MerkleProofCore<QHashOut<F>>,
    pub ups_cfc_standard_tx_whitelist_proof: MerkleProofCore<QHashOut<F>>,
    pub ups_cfc_deferred_tx_whitelist_proof: MerkleProofCore<QHashOut<F>>,

    // proof_tree_agg_circuits data
    pub single_leaf_circuit: QCommonCircuitData<F>,
    pub two_leaf_circuit: QCommonCircuitData<F>,
    pub two_agg_circuit: QCommonCircuitData<F>,
    pub left_leaf_right_agg_circuit: QCommonCircuitData<F>,
    pub left_agg_right_leaf_circuit: QCommonCircuitData<F>,
    pub leaf_circuit_config_id: u64,
    pub leaf_verifier_data_cap_height: usize,
    pub agg_verifier_data_cap_height: usize,

    pub circuit_inclusion_proofs: SimpleQTreeRecursionManagerInclusionProofs<F>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D> + 'static, const D: usize> ProveProxyRpcProvider<C, D>
where
    C::Hasher:AlgebraicHasher<C::F>,
{
    pub async fn new_with_config(proof_proxy_url: String) -> anyhow::Result<Self> {
        let client = Client::new();

        // todo fix bug
        // #[cfg(target_arch = "wasm32")]
        let response = client
            .post(&proof_proxy_url)
            .json(&RpcRequest {
                jsonrpc: Version::V2,
                request: RequestParams::<C::F>::GetCircuitsData(),
                id: Id::Number(1),
            })
            .send()
            .await?
            .json::<RpcResponse<String>>()
            .await?;
        // #[cfg(not(target_arch = "wasm32"))]
        // let response = client
        //     .post(&proof_proxy_url)
        //     .json(&RpcRequest {
        //         jsonrpc: Version::V2,
        //         request: RequestParams::<C::F>::GetCircuitsData(),
        //         id: Id::Number(1),
        //     })
        //     .send()?
        //     .json::<RpcResponse<String>>()?;
        let common_circuits_data = match response.result {
            ResponseResult::Success(common_circuits_data) => {
                tracing::info!("get common_circuits_data");
                serde_json::from_str(&common_circuits_data)?
            }
            ResponseResult::Error(e) => {
                return Err(anyhow::format_err!("rpc call failed `{:?}`", e))
            }
        };
        // let common_circuits_data = "";
        // let common_circuits_data = serde_json::from_str(&common_circuits_data)?;

        Ok(Self {
            client: Arc::new(client),
            common_circuits_data,
            zk_sign_inner_circuit: QEDBasicZKSignatureInnerCircuit::new(),
            proof_proxy_url,
            _marker: PhantomData,
        })
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D> + 'static, const D: usize> UPSCircuitManagerTrait<C, D> for ProveProxyRpcProvider<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        info_store.register_circuit(
            LocalCircuitType::UPSStart.into(),
            self.common_circuits_data.ups_start.fingerprint,
            self.common_circuits_data.ups_start.verifier_config.clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCStandard.into(),
            self.common_circuits_data.ups_cfc_standard_tx.fingerprint,
            self.common_circuits_data
                .ups_cfc_standard_tx
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.common_circuits_data.ups_cfc_deferred_tx.fingerprint,
            self.common_circuits_data
                .ups_cfc_deferred_tx
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.common_circuits_data.ups_end_cap.fingerprint,
            self.common_circuits_data
                .ups_end_cap
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::UPSEndCap.into(),
            self.common_circuits_data.ups_end_cap.fingerprint,
            self.common_circuits_data
                .ups_end_cap
                .verifier_config
                .clone(),
        );

        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSStart.into(),
            self.common_circuits_data.ups_start_whitelist_proof.clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCStandard.into(),
            self.common_circuits_data
                .ups_cfc_standard_tx_whitelist_proof
                .clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::UPSCFCDeferred.into(),
            self.common_circuits_data
                .ups_cfc_deferred_tx_whitelist_proof
                .clone(),
        );

        info_store.register_circuit(
            LocalCircuitType::PTAggSingle.into(),
            self.common_circuits_data.single_leaf_circuit.fingerprint,
            self.common_circuits_data
                .single_leaf_circuit
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggTwoLeaf.into(),
            self.common_circuits_data.two_leaf_circuit.fingerprint,
            self.common_circuits_data
                .two_leaf_circuit
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggTwoAgg.into(),
            self.common_circuits_data.two_agg_circuit.fingerprint,
            self.common_circuits_data
                .two_agg_circuit
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggLeftAggRightLeaf.into(),
            self.common_circuits_data
                .left_agg_right_leaf_circuit
                .fingerprint,
            self.common_circuits_data
                .left_agg_right_leaf_circuit
                .verifier_config
                .clone(),
        );
        info_store.register_circuit(
            LocalCircuitType::PTAggLeftLeafRightAgg.into(),
            self.common_circuits_data
                .left_leaf_right_agg_circuit
                .fingerprint,
            self.common_circuits_data
                .left_leaf_right_agg_circuit
                .verifier_config
                .clone(),
        );

        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::PTAggSingle.into(),
            self.common_circuits_data
                .circuit_inclusion_proofs
                .single_leaf_circuit_merkle_proof
                .clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::PTAggTwoLeaf.into(),
            self.common_circuits_data
                .circuit_inclusion_proofs
                .two_leaf_circuit_merkle_proof
                .clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::PTAggTwoAgg.into(),
            self.common_circuits_data
                .circuit_inclusion_proofs
                .two_agg_circuit_merkle_proof
                .clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::PTAggLeftAggRightLeaf.into(),
            self.common_circuits_data
                .circuit_inclusion_proofs
                .left_agg_right_leaf_circuit_merkle_proof
                .clone(),
        );
        info_store.register_whitelist_merkle_proof(
            LocalCircuitType::PTAggLeftLeafRightAgg.into(),
            self.common_circuits_data
                .circuit_inclusion_proofs
                .left_leaf_right_agg_circuit_merkle_proof
                .clone(),
        );
    }

    async fn prove_ups_start(
        &self,
        input: &UPSStartStepInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove ups start: {}", serde_json::to_string_pretty(&input)?);
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::ProveUpsStart(QProveUpsStartRPCRequest {
                input: input.clone(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn register_contract_circuits(
        &self,
        contract_id: u64,
        contract_code: &ContractCodeDefinition,
    ) -> anyhow::Result<()> {
        tracing::info!("register contract {} circuits", contract_id);
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::RegisterCircuits(QRegisterCircuitsRPCRequest {
                contract_id,
                contract_code: contract_code.clone(),
            }),
            ()
        );
        match response.result {
            ResponseResult::Success(_) => Ok(()),
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64> {
        tracing::info!("get method `{}` of contract {}", method_name, contract_id);
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::GetMethodId(QGetMethodIdRPCRequest {
                contract_id,
                method_name,
            }),
            u64
        );
        match response.result {
            ResponseResult::Success(method_id) => {
                tracing::info!("get method id `{}` of contract {}", method_id, contract_id);
                Ok(method_id)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        tracing::info!(
            "get method `{}` common data of contract {}",
            method_id,
            contract_id
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::GetContractMethodCommonData(
                QGetContractMethodCommonDataRPCRequest {
                    contract_id,
                    method_id,
                }
            ),
            QCommonCircuitData<C::F>
        );
        match response.result {
            ResponseResult::Success(data) => {
                tracing::info!(
                    "get method id `{}` of contract {}, fingerprint: {}, common data: {}",
                    method_id,
                    contract_id,
                    data.fingerprint.to_string(),
                    serde_json::to_string(&data.verifier_config)?,
                );
                Ok((data.fingerprint, data.verifier_config.to_verifier_data()))
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove contract call: {}",
            serde_json::to_string_pretty(&input)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::ProveContractCall(QProveContractCallRPCRequest {
                contract_id,
                method_id,
                input: input.clone(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove ups cfc standard tx: {}",
            serde_json::to_string_pretty(&input)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::UpsCfcStandardTx(QUpsCfcStandardTxRPCRequest {
                input: input.clone(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove ups cfc deferred tx: {}",
            serde_json::to_string_pretty(&input)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::UpsCfcDeferredTx(QUpsCfcDeferredTxRPCRequest {
                input: input.clone(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_zk_sign(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove_zk_sign: {}", sig_hash.to_string());
        let inner_proof = self.zk_sign_inner_circuit.prove_base(private_key, sig_hash)?;
        let inner_proof_str = serde_json::to_string(&inner_proof)?;

        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::ZKSignatureMinifierProof(QSignatureMinifierProofRPCRequest {
                inner_proof: inner_proof_str,
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_secp_sign(
        &self,
        signature: QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove_secp_sign: {}",
            serde_json::to_string_pretty(&signature)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::SECPSignatureProof(QSecpSignatureProofRPCRequest {
                signature,
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn register_software_defined_circuit(
        &self,
        input: SoftwareDefinedSignatureInput,
    ) -> anyhow::Result<QHashOut<C::F>> {
        tracing::info!("register_software_defined_circuit: ");
        let input = match input {
            SoftwareDefinedSignatureInput::QED(input) => input,
            SoftwareDefinedSignatureInput::PLONKY2(_) => unimplemented!(),
        };
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::RegisterSoftwareDefinedCircuit(
                QRegisterSoftwareDefinedCircuitRPCRequest { input }
            ),
            QHashOut<C::F>
        );
        match response.result {
            ResponseResult::Success(fingerprint) => {
                tracing::info!("get sdc fingerprint: {}", fingerprint.to_string());
                Ok(fingerprint)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: SoftwareDefinedSignatureWitnessInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove_software_defined_sign:");
        let input = match input {
            SoftwareDefinedSignatureWitnessInput::QED(input) => input,
            SoftwareDefinedSignatureWitnessInput::PLONKY2(_) => unimplemented!(),
        };
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParams::<C::F>::SoftwareDefinedSignatureProof(QSoftwareDefinedSignatureProofRPCRequest {
                fingerprint,
                private_key,
                input,
                sig_hash,
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove ups end cap: {}",
            serde_json::to_string_pretty(&end_cap_from_proof_tree_input)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C, D>::UpsEndCap(QUpsEndCapRPCRequestV2 {
                end_cap_from_proof_tree_input: end_cap_from_proof_tree_input.clone(),
                circuit_type: agg_proof_record.circuit_type,
                fingerprint: agg_proof_record.fingerprint,
                agg_header: agg_proof_record.agg_header,
                proof: agg_proof_record.proof.clone(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.ups_start.fingerprint)
    }

    async fn ups_start_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .ups_start
            .verifier_config
            .clone()
            .to_verifier_data())
    }

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.ups_cfc_standard_tx.fingerprint)
    }

    async fn ups_cfc_standard_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .ups_cfc_standard_tx
            .verifier_config
            .clone()
            .to_verifier_data())
    }

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.ups_cfc_deferred_tx.fingerprint)
    }

    async fn ups_cfc_deferred_tx_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .ups_cfc_deferred_tx
            .verifier_config
            .clone()
            .to_verifier_data::<C, D>())
    }

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.ups_end_cap.fingerprint)
    }

    async fn ups_end_cap_circuit_verifier_config(
        &self,
    ) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .ups_end_cap
            .verifier_config
            .to_verifier_data())
    }

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.ups_circuit_whitelist_root)
    }

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.zk_circuit.fingerprint)
    }

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .zk_circuit
            .verifier_config
            .clone()
            .to_verifier_data())
    }

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        Ok(self.common_circuits_data.secp_circuit.fingerprint)
    }

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        Ok(self
            .common_circuits_data
            .secp_circuit
            .verifier_config
            .clone()
            .to_verifier_data())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize> PortableQTreeRecursionCircuitsDataTrait<C, D>
    for ProveProxyRpcProvider<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn single_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.common_circuits_data.single_leaf_circuit.fingerprint
    }
    async fn two_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.common_circuits_data.two_leaf_circuit.fingerprint
    }
    async fn two_agg_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.common_circuits_data.two_agg_circuit.fingerprint
    }
    async fn left_leaf_right_agg_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.common_circuits_data
            .left_leaf_right_agg_circuit
            .fingerprint
    }
    async fn left_agg_right_leaf_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.common_circuits_data
            .left_agg_right_leaf_circuit
            .fingerprint
    }
    async fn single_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        self.common_circuits_data
            .single_leaf_circuit
            .verifier_config
            .clone()
            .to_verifier_data()
    }
    async fn two_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        self.common_circuits_data
            .two_leaf_circuit
            .verifier_config
            .clone()
            .to_verifier_data()
    }
    async fn two_agg_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        self.common_circuits_data
            .two_agg_circuit
            .verifier_config
            .clone()
            .to_verifier_data()
    }
    async fn left_leaf_right_agg_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        self.common_circuits_data
            .left_leaf_right_agg_circuit
            .verifier_config
            .clone()
            .to_verifier_data()
    }
    async fn left_agg_right_leaf_circuit_verifier_config(&self) -> VerifierOnlyCircuitData<C, D> {
        self.common_circuits_data
            .left_agg_right_leaf_circuit
            .verifier_config
            .clone()
            .to_verifier_data()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D> + 'static, const D: usize> PortableQTreeRecursionCircuitsProveTrait<C, D>
    for ProveProxyRpcProvider<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn get_verifier_data_by_type(
        &self,
        circuit_type: QStandardBinaryTreeCircuitType,
    ) -> VerifierOnlyCircuitData<C, D> {
        match circuit_type {
            QStandardBinaryTreeCircuitType::None => {
                panic!("tried to get verifier data for a circuit with type None")
            }
            QStandardBinaryTreeCircuitType::SingleLeaf => self
                .common_circuits_data
                .single_leaf_circuit
                .verifier_config
                .clone()
                .to_verifier_data(),
            QStandardBinaryTreeCircuitType::TwoLeaf => self
                .common_circuits_data
                .two_leaf_circuit
                .verifier_config
                .clone()
                .to_verifier_data(),
            QStandardBinaryTreeCircuitType::TwoAgg => self
                .common_circuits_data
                .two_agg_circuit
                .verifier_config
                .clone()
                .to_verifier_data(),
            QStandardBinaryTreeCircuitType::LeftLeafRightAgg => self
                .common_circuits_data
                .left_leaf_right_agg_circuit
                .verifier_config
                .clone()
                .to_verifier_data(),
            QStandardBinaryTreeCircuitType::LeftAggRightLeaf => self
                .common_circuits_data
                .left_agg_right_leaf_circuit
                .verifier_config
                .clone()
                .to_verifier_data(),
            QStandardBinaryTreeCircuitType::Root => {
                panic!("tried to get verifier data for a circuit with type Root")
            }
        }
    }
    async fn prove_single_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<C::F>,
        single_insert_leaf_proof: &DeltaMerkleProofCore<QHashOut<C::F>>,
        single_proof: &ProofWithPublicInputs<C::F, C, D>,
        single_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove single leaf circuit: {}",
            serde_json::to_string_pretty(&agg_circuit_whitelist_root)?
        );
        // todo fix bug
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C,D>::SingleLeaf(QSingleLeafRpcRequestV2 {
                agg_circuit_whitelist_root,
                single_insert_leaf_proof: single_insert_leaf_proof.clone(),
                single_proof: single_proof.clone(),
                single_verifier_data: single_verifier_data.into(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
    async fn prove_two_leaf_circuit(
        &self,
        agg_circuit_whitelist_root: QHashOut<C::F>,
        left_insert_leaf_proof: &DeltaMerkleProofCore<QHashOut<C::F>>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_insert_leaf_proof: &DeltaMerkleProofCore<QHashOut<C::F>>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!(
            "prove two leaf circuit: {}",
            serde_json::to_string_pretty(&agg_circuit_whitelist_root)?
        );
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C,D>::TwoLeaf(QTwoLeafRpcRequestV2{
                agg_circuit_whitelist_root,
                left_insert_leaf_proof: left_insert_leaf_proof.clone(),
                left_proof: left_proof.clone(),
                left_verifier_data: left_verifier_data.into(),
                right_insert_leaf_proof: right_insert_leaf_proof.clone(),
                right_proof: right_proof.clone(),
                right_verifier_data: right_verifier_data.into(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
    async fn prove_two_agg_circuit(
        &self,
        left_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        left_agg_proof_header: &QRecursionAggStandardHeader<C::F>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        right_agg_proof_header: &QRecursionAggStandardHeader<C::F>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove two agg circuit:");
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C,D>::TwoAgg(QTwoAggRpcRequsetV2{
                left_agg_whitelist_merkle_proof: left_agg_whitelist_merkle_proof.clone(),
                left_agg_proof_header: left_agg_proof_header.clone(),
                left_proof: left_proof.clone(),
                left_verifier_data: left_verifier_data.into(),
                right_agg_whitelist_merkle_proof: right_agg_whitelist_merkle_proof.clone(),
                right_agg_proof_header: right_agg_proof_header.clone(),
                right_proof: right_proof.clone(),
                right_verifier_data: right_verifier_data.into(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
    async fn prove_left_leaf_right_agg_circuit(
        &self,
        left_insert_leaf_proof: &DeltaMerkleProofCore<QHashOut<C::F>>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        right_agg_proof_header: &QRecursionAggStandardHeader<C::F>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove leaf leaf right agg circuit:");
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C, D>::LeftLeafRightAgg(QLeftLeafRightAggRpcRequestV2{
                left_insert_leaf_proof: left_insert_leaf_proof.clone(),
                left_proof: left_proof.clone(),
                left_verifier_data: left_verifier_data.into(),
                right_agg_whitelist_merkle_proof: right_agg_whitelist_merkle_proof.clone(),
                right_agg_proof_header: right_agg_proof_header.clone(),
                right_proof: right_proof.clone(),
                right_verifier_data: right_verifier_data.into(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
    async fn prove_left_agg_right_leaf_circuit(
        &self,
        left_agg_whitelist_merkle_proof: &MerkleProofCore<QHashOut<C::F>>,
        left_agg_proof_header: &QRecursionAggStandardHeader<C::F>,
        left_proof: &ProofWithPublicInputs<C::F, C, D>,
        left_verifier_data: &VerifierOnlyCircuitData<C, D>,
        right_insert_leaf_proof: &DeltaMerkleProofCore<QHashOut<C::F>>,
        right_proof: &ProofWithPublicInputs<C::F, C, D>,
        right_verifier_data: &VerifierOnlyCircuitData<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        tracing::info!("prove left agg right leaf circuit:");
        let response = qed_rpc_call_back!(
            self,
            &self.proof_proxy_url,
            RequestParamsV2::<C,D>::LeftAggRightLeaf(QLeftAggRightLeafRpcRequestV2{
                left_agg_whitelist_merkle_proof: left_agg_whitelist_merkle_proof.clone(),
                left_agg_proof_header: left_agg_proof_header.clone(),
                left_proof: left_proof.clone(),
                left_verifier_data: left_verifier_data.into(),
                right_insert_leaf_proof: right_insert_leaf_proof.clone(),
                right_proof: right_proof.clone(),
                right_verifier_data: right_verifier_data.into(),
            }),
            ProofWithPublicInputs<C::F, C, D>
        );
        match response.result {
            ResponseResult::Success(proof) => {
                tracing::info!(
                    "get proof: {}",
                    serde_json::to_string_pretty(&proof.public_inputs)?
                );
                Ok(proof)
            }
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D> + 'static, const D: usize> PortableQTreeRecursionCircuitsTrait<C, D>
    for ProveProxyRpcProvider<C, D>
where
    C::Hasher:
        AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn circuit_inclusion_proofs(&self) -> &SimpleQTreeRecursionManagerInclusionProofs<C::F> {
        &self.common_circuits_data.circuit_inclusion_proofs
    }
}
