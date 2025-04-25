use std::sync::atomic::Ordering;
use jsonrpsee::RpcModule;
use jsonrpsee::types::ErrorObjectOwned;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_store::config::store_config::QEDFelt;
use crate::edge::context::LATEST_CHECKPOINT_ID;
use crate::edge::rpc::handler::CoordinatorEdgeHandler;
use crate::edge::rpc::types::{GetUserIdRequest, SubmitGUTAParams};

/// register the RPC methods for the CoordinatorEdgeHandler
pub fn build_rpc_module(
    redis_uri: &str,
) -> anyhow::Result<(RpcModule<CoordinatorEdgeHandler>, CoordinatorEdgeHandler)> {
    let handler = CoordinatorEdgeHandler::new(redis_uri)?;
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

    Ok((module, handler_clone))
}