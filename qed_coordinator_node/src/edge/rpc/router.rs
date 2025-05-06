use std::sync::atomic::Ordering;
use jsonrpsee::RpcModule;
use qed_core::data::qhashout::QHashOut;
use jsonrpsee::types::{ErrorObject, ErrorObjectOwned, Params};
use plonky2::hash::hash_types::RichField;
use serde::de::DeserializeOwned;
use serde::Serialize;
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState};
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_realm_node::{C, F};
use qed_store::config::store_config::QEDFelt;
use crate::context::{REGISTERED_USERS};
use crate::edge::context::LATEST_CHECKPOINT_ID;
use crate::edge::rpc::handler::CoordinatorEdgeHandler;
use crate::edge::rpc::types::{GetUserIdRequest, SubmitGUTAParams};
use crate::rpc::types::{GetByFRequest, GetByIdRequest, GetUserLeafRequest, GetUserRegistrationFLeafRequest, GetUserRegistrationLeafRequest, LatestCheckpointResponse, RealmInfo, RegisterRealmRpcRequest};

/// register the RPC methods for the CoordinatorEdgeHandler
pub fn build_rpc_module(
    args: CoordinatorEdgeArgs,
) -> anyhow::Result<(RpcModule<CoordinatorEdgeHandler>, CoordinatorEdgeHandler)> {
    let handler = CoordinatorEdgeHandler::new(args.clone())?;
    let handler_clone = handler.clone();

    let mut module = RpcModule::new(handler);

    //qed_register_user
    module.register_async_method("qed_register_user", |params, handler, _ext| async move {
        tracing::info!(
            "➡️ Received method = register_user, raw params = {:?}",
            params
        );

        let pub_key = match params.parse::<ZKPublicKeyInfo<QEDFelt>>() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("❌ Failed to parse params for register_user: {}", e);
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.register_user(pub_key).await {
            Ok(_) => {
                // tracing::info!("✅ register_user success");
                Ok::<_, ErrorObjectOwned>("ok")
            }
            Err(e) => {
                tracing::error!("❌ register_user failed: {:?}", e);
                Err(ErrorObjectOwned::owned(1, e.to_string(), None::<()>))
            }
        }
    })?;

    module.register_async_method("qed_get_user_id", |params, handler, _ext| async move {
        tracing::info!(
            "➡️ Received method = qed_get_user_id, raw params = {:?}",
            params
        );

        let qhash: QHashOut<QEDFelt> = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("❌ Failed to parse params: {}", e);
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params: {}", e),
                    None::<()>,
                ));
            }
        };


        match handler.get_user_id_logic(qhash).await {
            Ok(user_id) => Ok(user_id),
            Err(e) => {
                tracing::error!("❌ get_user_id_logic failed: {:?}", e);

                let msg = e.to_string();
                let code = if msg.contains("User not found") {
                    -32004
                } else {
                    -32005
                };

                Err(ErrorObjectOwned::owned(code, msg, None::<()>))
            }
        }
    })?;
    //qed_deploy_contract
    module.register_async_method("qed_deploy_contract", |params, handler, _ext| async move {
        let contract = params.parse().map_err(|e| {
            ErrorObjectOwned::owned(
                -32602,
                format!("Invalid contract params: {}", e),
                None::<()>,
            )
        })?;

        handler
            .deploy_contract(contract)
            .await
            .map_err(|e| ErrorObjectOwned::owned(2, e.to_string(), None::<()>))?;

        Ok::<_, ErrorObjectOwned>("ok")
    })?;

    // qed_submit_guta
    module.register_async_method("qed_submit_guta", |params, handler, ext| async move {
        // tracing::info!(?params, "qed_submit_guta");

        // let jwt_metadata = ext.get::<JwtAuthMetadata>()
        //     .ok_or_else(|| ErrorObjectOwned::owned(401, "Missing JwtAuthMetadata", None::<()>))?;

        // validate_jwt_from_ext(&jwt_metadata)?;

        let SubmitGUTAParams { input, proof } = params.parse().map_err(|e| {
            ErrorObjectOwned::owned(-32602, format!("Invalid GUTA input: {}", e), None::<()>)
        })?;
        handler
            .submit_guta(input, proof)
            .await
            .map_err(|e| ErrorObjectOwned::owned(3, e.to_string(), None::<()>))?;

        Ok::<_, ErrorObjectOwned>("ok")
    })?;
    //register_realm_rpc
    // module.register_async_method("register_realm_rpc", move |params, _handler, _ctx| {
    //     let args = args.clone();
    //     async move {
    //         let parsed: RegisterRealmRpcRequest = match params.parse() {
    //             Ok(p) => p,
    //         Err(e) => {
    //             return Err(ErrorObjectOwned::owned(
    //                 -32602,
    //                 format!("Invalid params for register_realm_rpc: {}", e),
    //                 None::<()>,
    //             ));
    //         }
    //     };
    //
    //     let mut registry = GLOBAL_REALM_REGISTRY.write().await;
    //     if !registry.realms.contains_key(&parsed.rpc_url) {
    //         tracing::info!("✅ Realm registered via RPC: name = {}, url = {}", parsed.name, parsed.rpc_url);
    //
    //         let latest_checkpoint_id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);
    //         fetch_and_push_checkpoint_to_realm::<F,C,2>(args.clone().coordinator_edge_queue_args, latest_checkpoint_id, &parsed.rpc_url)
    //             .await
    //             .map_err(|e| {
    //                 tracing::warn!("❌ fetch_and_push_checkpoint_to_realm failed: {:?}", e);
    //                 ErrorObject::owned(-32000, format!("Failed to push checkpoint: {}", e), None::<()>)
    //             })?;
    //         registry.realms.insert(parsed.rpc_url.clone(), RealmInfo {
    //             name: parsed.name,
    //             rpc_url: parsed.rpc_url,
    //         });
    //     } else {
    //         tracing::info!("ℹ️ Realm already exists: {}", parsed.rpc_url);
    //         }
    //
    //         Ok(serde_json::Value::Bool(true))
    //     }
    // })?;
    //qed_get_latest_checkpoint
    module.register_async_method("qed_get_latest_checkpoint", |_params, _handler, _ext| async move {
        let checkpoint = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);
        let response = LatestCheckpointResponse {
            checkpoint_id: checkpoint,
        };
        Ok::<_, ErrorObjectOwned>(response)
    })?;

    // qed_build_block
    module.register_async_method("qed_build_block", |_params, handler, _ext| async move {
        handler
            .build_block()
            .await
            .map_err(|e| ErrorObjectOwned::owned(1, e.to_string(), None::<()>))?;

        Ok::<_, ErrorObjectOwned>("ok")
    })?;

    module.register_async_method("qed_get_checkpoint_sync_info", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = match params.parse::<(u64,)>() {
            Ok((id,)) => id,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid checkpoint_id for get_checkpoint_info: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_checkpoint_sync_info(checkpoint_id).await {
            Ok(sync_info) => Ok(serde_json::to_value(&sync_info).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_checkpoint_sync_info error: {:?}", e);
                Err(ErrorObjectOwned::owned(7, e.to_string(), None::<()>))
            }
        }
    })?;


    // async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    module.register_async_method("qed_get_contract_leaf_data", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params for contract leaf: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_contract_leaf_data(parsed.id).await {
            Ok(leaf) => Ok(serde_json::to_value(&leaf).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_contract_leaf_data error: {:?}", e);
                Err(ErrorObjectOwned::owned(6, e.to_string(), None::<()>))
            }
        }
    })?;
    // async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;
    module.register_async_method("get_contract_leaf_data_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params for contract leaf: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_contract_leaf_data_f(parsed.id).await {
            Ok(leaf) => Ok(serde_json::to_value(&leaf).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_contract_leaf_data error: {:?}", e);
                Err(ErrorObjectOwned::owned(6, e.to_string(), None::<()>))
            }
        }
    })?;
    //
    // async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    module.register_async_method("qed_get_checkpoint_leaf_data", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params for checkpoint leaf: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_checkpoint_leaf_data(parsed.id).await {
            Ok(leaf) => Ok(serde_json::to_value(&leaf).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_checkpoint_leaf_data error: {:?}", e);
                Err(ErrorObjectOwned::owned(7, e.to_string(), None::<()>))
            }
        }
    })?;

    // async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    module.register_async_method("get_checkpoint_leaf_data_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "contract leaf")?;

        match handler.get_checkpoint_leaf_data_f(parsed.id).await {
            Ok(leaf) => Ok(serde_json::to_value(&leaf).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_contract_leaf_data error: {:?}", e);
                Err(ErrorObjectOwned::owned(6, e.to_string(), None::<()>))
            }
        }
    })?;
    // async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    //get_contract_code_definition
    module.register_async_method("qed_get_contract_code_definition", |params, handler, _ctx| async move {
        tracing::info!("➡️ Received method = qed_get_contract_code_definition, params = {:?}", params);

        let parsed: GetByIdRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("❌ Failed to parse qed_get_contract_code_definition params: {}", e);
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_contract_code_definition(parsed.id).await {
            Ok(code_def) => Ok(serde_json::to_value(&code_def).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_contract_code_definition error: {:?}", e);
                Err(ErrorObjectOwned::owned(5, e.to_string(), None::<()>))
            }
        }
    })?;
    // async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;
    module.register_async_method("get_contract_code_definition_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "contract definition")?;

        handle_rpc_result(
            handler.get_contract_code_definition_f(parsed.id),
            6,
            "get_contract_leaf_data",
        ).await
    })?;
    // async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
    module.register_async_method("qed_latest_l2_block_state", |_params, handler, _ctx| async move {
        match handler.get_latest_l2_block_state().await {
            Ok(state) => {
                Ok::<_, ErrorObjectOwned>(serde_json::to_value(&state).unwrap())
            }
            Err(e) => {
                tracing::error!("❌ qed_latest_l2_block_state error: {:?}", e);
                Err(ErrorObjectOwned::owned(4, e.to_string(), None::<()>))
            }
        }
    })?;
    // async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    module.register_async_method("qed_get_l2_block_state", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params for qed_get_l2_block_state: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_l2_block_state(parsed.id).await {
            Ok(state) => Ok(serde_json::to_value(&state).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_l2_block_state error: {:?}", e);
                Err(ErrorObjectOwned::owned(8, e.to_string(), None::<()>))
            }
        }
    })?;
    // async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
    module.register_async_method("get_l2_block_state_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "l2_block_state")?;

        handle_rpc_result(
            handler.get_l2_block_state_f(parsed.id),
            6,
            "get_contract_leaf_data",
        ).await
    })?;
    //
    // async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_registration_tree_root", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = parse_params(params, "get_user_registration_tree_root")?;

        handle_rpc_result(
            handler.get_user_registration_tree_root(parsed.id),
            6,
            "get_contract_leaf_data",
        ).await
    })?;
    // async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_registration_tree_root_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "get_user_registration_tree_root_f")?;

        handle_rpc_result(
            handler.get_user_registration_tree_root_f(parsed.id),
            6,
            "get_user_registration_tree_root_f",
        ).await
    })?;
    // async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_registration_tree_leaf_hash", |params, handler, _ctx| async move {
        let parsed: GetUserRegistrationLeafRequest = parse_params(params, "get_user_registration_tree_leaf_hash")?;

        handle_rpc_result(
            handler.get_user_registration_tree_leaf_hash(parsed.checkpoint_id, parsed.leaf_index),
            6,
            "get_user_registration_tree_leaf_hash",
        ).await
    })?;
    // async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_registration_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let parsed: GetUserRegistrationFLeafRequest = parse_params(params, "get_user_registration_tree_leaf_hash_f")?;

        handle_rpc_result(
            handler.get_user_registration_tree_leaf_hash_f(parsed.checkpoint_id, parsed.leaf_index),
            6,
            "get_contract_leaf_data",
        ).await
    })?;
    // async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_user_registration_tree_merkle_proof", |params, handler, _ctx| async move {
        let parsed: GetUserRegistrationLeafRequest = parse_params(params, "get_user_registration_tree_merkle_proof")?;

        handle_rpc_result(
            handler.get_user_registration_tree_merkle_proof(parsed.checkpoint_id, parsed.leaf_index),
            6,
            "get_contract_leaf_data",
        ).await
    })?;
    // async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_user_registration_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let parsed: GetUserRegistrationFLeafRequest = parse_params(params, "get_user_registration_tree_merkle_proof_f")?;

        handle_rpc_result(
            handler.get_user_registration_tree_merkle_proof_f(parsed.checkpoint_id, parsed.leaf_index),
            6,
            "get_contract_leaf_data",
        ).await
    })?;

    //
    // async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_tree_root", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = parse_params(params, "get_user_tree_root")?;

        handle_rpc_result(
            handler.get_user_tree_root(parsed.id),
            6,
            "get_user_tree_root",
        ).await
    })?;
    // async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_tree_root_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "get_user_tree_root_f")?;

        handle_rpc_result(
            handler.get_user_tree_root_f(parsed.id),
            6,
            "get_user_tree_root_f",
        ).await
    })?;
    // async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_user_sub_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, root_level, leaf_level, leaf_index): (u64, u8, u8, u64) = parse_params(params, "get_user_sub_tree_merkle_proof")?;

        handle_rpc_result(
            handler.get_user_sub_tree_merkle_proof(
                checkpoint_id,
                root_level,
                leaf_level,
                leaf_index,
            ),
            6,
            "get_user_sub_tree_merkle_proof",
        ).await
    })?;

    // async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_user_top_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, leaf_level, leaf_index): (u64, u8, u64) = parse_params(params, "get_user_top_tree_merkle_proof")?;

        handle_rpc_result(
            handler.get_user_top_tree_merkle_proof(checkpoint_id, leaf_level, leaf_index),
            6,
            "get_user_top_tree_merkle_proof",
        ).await
    })?;

    // async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_top_tree_cap_root", |params, handler, _ctx| async move {
        let (checkpoint_id, cap_level, cap_index): (u64, u8, u64) = parse_params(params, "get_user_top_tree_cap_root")?;

        handle_rpc_result(
            handler.get_user_top_tree_cap_root(checkpoint_id, cap_level, cap_index),
            6,
            "get_user_top_tree_cap_root",
        ).await
    })?;
    // async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_user_latest_top_tree_cap_root", |params, handler, _ctx| async move {
        let (cap_level, cap_index): (u8, u64) = parse_params(params, "get_user_latest_top_tree_cap_root")?;

        handle_rpc_result(
            handler.get_user_latest_top_tree_cap_root(cap_level, cap_index),
            6,
            "get_user_latest_top_tree_cap_root",
        ).await
    })?;
    //
    //
    // async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_function_tree_root", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (u64, u32) = parse_params(params, "get_contract_function_tree_root")?;

        handle_rpc_result(
            handler.get_contract_function_tree_root(checkpoint_id, contract_id),
            6,
            "get_contract_function_tree_root",
        ).await
    })?;
    // async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_function_tree_root_f", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (F, F) = parse_params(params, "get_contract_function_tree_root_f")?;

        handle_rpc_result(
            handler.get_contract_function_tree_root_f(checkpoint_id, contract_id),
            6,
            "get_contract_function_tree_root_f",
        ).await
    })?;
    // async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_function_tree_leaf_hash", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id, function_id): (u64, u32, u32) = parse_params(params, "get_contract_function_tree_leaf_hash")?;

        handle_rpc_result(
            handler.get_contract_function_tree_leaf_hash(checkpoint_id, contract_id, function_id),
            6,
            "get_contract_function_tree_leaf_hash",
        ).await
    })?;

    // async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_function_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id, function_id): (F, F, F) = parse_params(params, "get_contract_function_tree_leaf_hash_f")?;

        handle_rpc_result(
            handler.get_contract_function_tree_leaf_hash_f(checkpoint_id, contract_id, function_id),
            6,
            "get_contract_function_tree_leaf_hash_f",
        ).await
    })?;
    // async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_contract_function_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id, function_id): (u64, u32, u32) = parse_params(params, "get_contract_function_tree_merkle_proof")?;

        handle_rpc_result(
            handler.get_contract_function_tree_merkle_proof(checkpoint_id, contract_id, function_id),
            6,
            "get_contract_function_tree_merkle_proof",
        ).await
    })?;
    // async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_contract_function_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id, function_id): (F, F, F) = parse_params(params, "get_contract_function_tree_merkle_proof_f")?;

        handle_rpc_result(
            handler.get_contract_function_tree_merkle_proof_f(checkpoint_id, contract_id, function_id),
            6,
            "get_contract_function_tree_merkle_proof_f",
        ).await
    })?;

    // async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_tree_root", |params, handler, _ctx| async move {
        let parsed: GetByIdRequest = parse_params(params, "get_contract_tree_root")?;

        handle_rpc_result(
            handler.get_contract_tree_root(parsed.id),
            6,
            "get_contract_tree_root",
        ).await
    })?;
    // async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_tree_root_f", |params, handler, _ctx| async move {
        let parsed: GetByFRequest = parse_params(params, "get_contract_tree_root_f")?;

        handle_rpc_result(
            handler.get_contract_tree_root_f(parsed.id),
            6,
            "get_contract_tree_root_f",
        ).await
    })?;
    // async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_tree_leaf_hash", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (u64, u32) = parse_params(params, "get_contract_tree_leaf_hash")?;

        handle_rpc_result(
            handler.get_contract_tree_leaf_hash(checkpoint_id, contract_id),
            6,
            "get_contract_tree_leaf_hash",
        ).await
    })?;
    // async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_contract_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (F, F) = parse_params(params, "get_contract_tree_leaf_hash_f")?;

        handle_rpc_result(
            handler.get_contract_tree_leaf_hash_f(checkpoint_id, contract_id),
            6,
            "get_contract_tree_leaf_hash_f",
        ).await
    })?;

    // async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_contract_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (u64, u32) = parse_params(params, "get_contract_tree_merkle_proof")?;

        handle_rpc_result(
            handler.get_contract_tree_merkle_proof(checkpoint_id, contract_id),
            6,
            "get_contract_tree_merkle_proof",
        ).await
    })?;
    // async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_contract_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let (checkpoint_id, contract_id): (F, F) = parse_params(params, "get_contract_tree_merkle_proof_f")?;

        handle_rpc_result(
            handler.get_contract_tree_merkle_proof_f(checkpoint_id, contract_id),
            6,
            "get_contract_tree_merkle_proof_f",
        ).await
    })?;
    //
    //
    //
    // async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_deposit_tree_root", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = parse_params(params, "get_deposit_tree_root")?;
        handle_rpc_result(
            handler.get_deposit_tree_root(checkpoint_id),
            6,
            "get_deposit_tree_root",
        ).await
    })?;
    // async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_deposit_tree_root_f", |params, handler, _ctx| async move {
        let checkpoint_id: F = parse_params(params, "get_deposit_tree_root_f")?;
        handle_rpc_result(
            handler.get_deposit_tree_root_f(checkpoint_id),
            6,
            "get_deposit_tree_root_f",
        ).await
    })?;
    // async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_deposit_tree_leaf_hash", |params, handler, _ctx| async move {
        let (checkpoint_id, deposit_id): (u64, u32) = parse_params(params, "get_deposit_tree_leaf_hash")?;
        handle_rpc_result(
            handler.get_deposit_tree_leaf_hash(checkpoint_id, deposit_id),
            6,
            "get_deposit_tree_leaf_hash",
        ).await
    })?;
    // async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_deposit_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let (checkpoint_id, deposit_id): (F, F) = parse_params(params, "get_deposit_tree_leaf_hash_f")?;
        handle_rpc_result(
            handler.get_deposit_tree_leaf_hash_f(checkpoint_id, deposit_id),
            6,
            "get_deposit_tree_leaf_hash_f",
        ).await
    })?;
    // async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_deposit_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, deposit_id): (u64, u32) = parse_params(params, "get_deposit_tree_merkle_proof")?;
        handle_rpc_result(
            handler.get_deposit_tree_merkle_proof(checkpoint_id, deposit_id),
            6,
            "get_deposit_tree_merkle_proof",
        ).await
    })?;
    // async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_deposit_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let (checkpoint_id, deposit_id): (F, F) = parse_params(params, "get_deposit_tree_merkle_proof_f")?;
        handle_rpc_result(
            handler.get_deposit_tree_merkle_proof_f(checkpoint_id, deposit_id),
            6,
            "get_deposit_tree_merkle_proof_f",
        ).await
    })?;
    //
    //
    // async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_withdrawal_tree_root", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = parse_params(params, "get_withdrawal_tree_root")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_root(checkpoint_id),
            6,
            "get_withdrawal_tree_root",
        ).await
    })?;
    // async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_withdrawal_tree_root_f", |params, handler, _ctx| async move {
        let checkpoint_id: F = parse_params(params, "get_withdrawal_tree_root_f")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_root_f(checkpoint_id),
            6,
            "get_withdrawal_tree_root_f",
        ).await
    })?;
    // async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_withdrawal_tree_leaf_hash", |params, handler, _ctx| async move {
        let (checkpoint_id, withdrawal_id): (u64, u32) = parse_params(params, "get_withdrawal_tree_leaf_hash")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_leaf_hash(checkpoint_id, withdrawal_id),
            6,
            "get_withdrawal_tree_leaf_hash",
        ).await
    })?;
    // async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_withdrawal_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let (checkpoint_id, withdrawal_id): (F, F) = parse_params(params, "get_withdrawal_tree_leaf_hash_f")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_leaf_hash_f(checkpoint_id, withdrawal_id),
            6,
            "get_withdrawal_tree_leaf_hash_f",
        ).await
    })?;
    // async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_withdrawal_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, withdrawal_id): (u64, u32) = parse_params(params, "get_withdrawal_tree_merkle_proof")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_merkle_proof(checkpoint_id, withdrawal_id),
            6,
            "get_withdrawal_tree_merkle_proof",
        ).await
    })?;
    // async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_withdrawal_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let (checkpoint_id, withdrawal_id): (F, F) = parse_params(params, "get_withdrawal_tree_merkle_proof_f")?;
        handle_rpc_result(
            handler.get_withdrawal_tree_merkle_proof_f(checkpoint_id, withdrawal_id),
            6,
            "get_withdrawal_tree_merkle_proof_f",
        ).await
    })?;
    //
    // async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_latest_checkpoint_tree_root", |_params, handler, _ctx| async move {
        handle_rpc_result(
            handler.get_latest_checkpoint_tree_root(),
            6,
            "get_latest_checkpoint_tree_root",
        ).await
    })?;
    // async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_checkpoint_tree_root", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = parse_params(params, "get_checkpoint_tree_root")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_root(checkpoint_id),
            6,
            "get_checkpoint_tree_root",
        ).await
    })?;
    // async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_checkpoint_tree_root_f", |params, handler, _ctx| async move {
        let checkpoint_id: F = parse_params(params, "get_checkpoint_tree_root_f")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_root_f(checkpoint_id),
            6,
            "get_checkpoint_tree_root_f",
        ).await
    })?;
    // async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_checkpoint_tree_leaf_hash", |params, handler, _ctx| async move {
        let (checkpoint_id, leaf_checkpoint_id): (u64, u64) = parse_params(params, "get_checkpoint_tree_leaf_hash")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_leaf_hash(checkpoint_id, leaf_checkpoint_id),
            6,
            "get_checkpoint_tree_leaf_hash",
        ).await
    })?;
    // async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    module.register_async_method("get_checkpoint_tree_leaf_hash_f", |params, handler, _ctx| async move {
        let (checkpoint_id, leaf_checkpoint_id): (F, F) = parse_params(params, "get_checkpoint_tree_leaf_hash_f")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_leaf_hash_f(checkpoint_id, leaf_checkpoint_id),
            6,
            "get_checkpoint_tree_leaf_hash_f",
        ).await
    })?;
    // async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_checkpoint_tree_merkle_proof", |params, handler, _ctx| async move {
        let (checkpoint_id, leaf_checkpoint_id): (u64, u64) = parse_params(params, "get_checkpoint_tree_merkle_proof")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_merkle_proof(checkpoint_id, leaf_checkpoint_id),
            6,
            "get_checkpoint_tree_merkle_proof",
        ).await
    })?;
    // async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    module.register_async_method("get_checkpoint_tree_merkle_proof_f", |params, handler, _ctx| async move {
        let (checkpoint_id, leaf_checkpoint_id): (F, F) = parse_params(params, "get_checkpoint_tree_merkle_proof_f")?;
        handle_rpc_result(
            handler.get_checkpoint_tree_merkle_proof_f(checkpoint_id, leaf_checkpoint_id),
            6,
            "get_checkpoint_tree_merkle_proof_f",
        ).await
    })?;
    //
    // async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    module.register_async_method("get_checkpoint_global_state_roots", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = parse_params(params, "get_checkpoint_global_state_roots")?;
        handle_rpc_result(
            handler.get_checkpoint_global_state_roots(checkpoint_id),
            6,
            "get_checkpoint_global_state_roots",
        ).await
    })?;
    // async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;
    module.register_async_method("get_checkpoint_sync_info_compact", |params, handler, _ctx| async move {
        let checkpoint_id: u64 = parse_params(params, "get_checkpoint_sync_info_compact")?;

        handle_rpc_result(
            handler.get_checkpoint_sync_info_compact(checkpoint_id),
            6,
            "get_checkpoint_sync_info_compact",
        ).await
    })?;

    //qed_latest_checkpoint
    module.register_async_method("qed_latest_checkpoint", |_params, _handler, _ctx| async move {
        let id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);
        Ok::<_, ErrorObjectOwned>(id)
    })?;
    module.register_async_method("qed_get_user_leaf_data", |params, handler, _ctx| async move {
        let parsed: GetUserLeafRequest = match params.parse() {
            Ok(p) => p,
            Err(e) => {
                return Err(ErrorObjectOwned::owned(
                    -32602,
                    format!("Invalid params for qed_get_user_leaf_data: {}", e),
                    None::<()>,
                ));
            }
        };

        match handler.get_user_leaf_data(parsed.checkpoint_id, parsed.user_id).await {
            Ok(leaf) => Ok(serde_json::to_value(&leaf).unwrap()),
            Err(e) => {
                tracing::error!("❌ get_user_leaf_data error: {:?}", e);
                Err(ErrorObjectOwned::owned(9, e.to_string(), None::<()>))
            }
        }
    })?;









    Ok((module, handler_clone))
}


pub fn parse_params<T: DeserializeOwned>(
    params: Params,
    context: &'static str,
) -> Result<T, ErrorObjectOwned> {
    params.parse::<T>().map_err(|e| {
        ErrorObjectOwned::owned(
            -32602,
            format!("Invalid params for {}: {}", context, e),
            None::<()>,
        )
    })
}

pub async fn handle_rpc_result<T, Fut>(
    fut: Fut,
    error_code: i32,
    context: &'static str,
) -> Result<serde_json::Value, ErrorObjectOwned>
where
    T: Serialize,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    match fut.await {
        Ok(res) => Ok(serde_json::to_value(&res).unwrap()),
        Err(e) => {
            tracing::error!("❌ {} error: {:?}", context, e);
            Err(ErrorObjectOwned::owned(error_code, e.to_string(), None::<()>))
        }
    }
}


use jsonrpsee::types::Request;
use kvq::traits::KVQSerializable;
use qed_node::coordinator::state::user_map::{get_node_redis_pool, get_user_id_by_pubkey};
use qed_rollup_utils::{decrypt_jwt_token, Claims};
use crate::{CoordinatorEdgeArgs, CoordinatorProcessorQueueArgs};

// pub fn validate_jwt_from_ext(ext: &JwtAuthMetadata) -> Result<(), ErrorObjectOwned> {
//
//     let jwt_meta = ext;
//     let token = &jwt_meta.token;
//
//     let secret = get_global_jwt_secret();
//     match decrypt_jwt_token(&secret, token) {
//         Ok(claims) => {
//             tracing::info!("✅ Valid JWT, realm_id = {}", claims.realm_id);
//             Ok(())
//         }
//         Err(e) => {
//             tracing::warn!("❌ Invalid JWT token: {:?}", e);
//             Err(ErrorObjectOwned::owned(401, format!("Invalid token: {}", e), None::<()>))
//         }
//     }
// }

#[derive(Clone, Debug)]
pub struct JwtAuthMetadata {
    pub token: String,
    pub realm_id: u64,
}
pub fn claims_to_auth_metadata(token: String, claims: Claims) -> JwtAuthMetadata {
    JwtAuthMetadata {
        token,
        realm_id: claims.realm_id,
    }
}