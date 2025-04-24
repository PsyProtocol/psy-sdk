use std::sync::atomic::Ordering;
use jsonrpsee::RpcModule;
use jsonrpsee::types::ErrorObjectOwned;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;
use crate::coordinator_edge::context::LATEST_CHECKPOINT_ID;
use crate::coordinator_edge::rpc::handler::CoordinatorEdgeHandler;
use crate::coordinator_edge::rpc::types::{GetByIdRequest, GetUserIdRequest, GetUserLeafRequest, SubmitGUTAParams};

/// register the RPC methods for the CoordinatorEdgeHandler
pub fn build_rpc_module(
    redis_url: &str,
) -> anyhow::Result<(RpcModule<CoordinatorEdgeHandler>, CoordinatorEdgeHandler)> {
    let handler = CoordinatorEdgeHandler::new(redis_url)?;
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
                tracing::info!("✅ register_user success");
                Ok(())
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

        let parsed: GetUserIdRequest = match params.parse() {
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

        match handler.get_user_id_by_pub_key(parsed).await {
            Ok(Some(user_id)) => {
                tracing::info!("✅ user found, user_id = {}", user_id);
                Ok(serde_json::json!({ "user_id": user_id }))
            }
            Ok(None) => {
                tracing::info!("🛑 user not found");
                Ok(serde_json::json!({ "user_id": null }))
            }
            Err(e) => {
                tracing::error!("❌ error in get_user_id_by_pubkey: {:?}", e);
                Err(ErrorObjectOwned::owned(1, e.to_string(), None::<()>))
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
    module.register_async_method("qed_submit_guta", |params, handler, _ext| async move {
        let SubmitGUTAParams { input, proof } = params.parse().map_err(|e| {
            ErrorObjectOwned::owned(-32602, format!("Invalid GUTA input: {}", e), None::<()>)
        })?;

        handler
            .submit_guta(input, proof)
            .await
            .map_err(|e| ErrorObjectOwned::owned(3, e.to_string(), None::<()>))?;

        Ok::<_, ErrorObjectOwned>("ok")
    })?;


    // qed_build_block
    module.register_async_method("qed_build_block", |_params, handler, _ext| async move {
        handler
            .build_block()
            .await
            .map_err(|e| ErrorObjectOwned::owned(1, e.to_string(), None::<()>))?;

        Ok::<_, ErrorObjectOwned>("ok")
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

    //get_contract_leaf_data
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
    //get_checkpoint_leaf_data
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

    Ok((module, handler_clone))
}