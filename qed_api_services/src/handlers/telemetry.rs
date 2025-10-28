use std::cmp::min;
use std::sync::Arc;
use axum::{extract::State, http::StatusCode, middleware, response::Json, routing::post, Extension, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use crate::{models::*, repositories::*, services::ApiService};
use crate::auth::{auth_middleware, AuthExtension, JwtManager};
use crate::repositories::checkpoint_state::{CheckpointStatsRepository, WorkerJobEventRepository};

pub fn create_telemetry_router(api_service: ApiService, jwt_manager: Arc<JwtManager>) -> Router {
    Router::new()
        .route("/telemetry/events", post(receive_events_handler))
        .route("/telemetry/checkpoint/leaves", post(report_checkpoint_leafs_handler))
        .layer(middleware::from_fn_with_state(
            jwt_manager.clone(),
            auth_middleware,
        ))
        .with_state(api_service)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub worker_events: Option<Vec<WorkerEvent>>,
    pub user_events: Option<Vec<UserEvent>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryResponse {
    pub success: bool,
    pub processed_count: usize,
    pub service: String,
}

async fn receive_events_handler(
    State(service): State<ApiService>,
    Extension(auth): Extension<AuthExtension>, // Extract authenticated claims
    Json(payload): Json<TelemetryPayload>,
) -> Result<Json<TelemetryResponse>, StatusCode> {
    info!(
        "Telemetry events received from '{}': worker_events={}, user_events={}",
        auth.claims.sub,
        payload.worker_events.as_ref().map(|v| v.len()).unwrap_or(0),
        payload.user_events.as_ref().map(|v| v.len()).unwrap_or(0)
    );
    info!("Telemetry payload: {:?}", payload);

    let mut processed_count = 0;

    // Process worker events
    if let Some(ref worker_events) = payload.worker_events {
        info!("Processing {} worker events", worker_events.len());
        for (index, worker_event) in worker_events.iter().enumerate() {
            info!(
                "Processing worker event {}/{}: {:?}",
                index + 1,
                worker_events.len(),
                worker_event
            );
            match WorkerEventRepository::create(
                &service.pool,
                worker_event.realm_id,
                worker_event.public_key.as_deref(),
                worker_event.status.clone(),
                worker_event.source.clone(),
                &worker_event.job_id,
                worker_event.checkpoint_id,
                worker_event.duration,
                worker_event.metadata.as_ref(),
                worker_event.timestamp,
            )
            .await
            {
                Ok(_) => {
                    processed_count += 1;
                    info!(
                        "Worker event inserted successfully: job_id={:?}",
                        worker_event.job_id
                    );
                }
                Err(e) => {
                    error!("Failed to insert worker event: {:?}", e);
                    continue;
                }
            }
        }
    }

    // Process user events
    if let Some(ref user_events) = payload.user_events {
        info!("Processing {} user events", user_events.len());
        for (index, user_event) in user_events.iter().enumerate() {
            info!(
                "Processing user event {}/{}: {:?}",
                index + 1,
                user_events.len(),
                user_event
            );
            match UserEventRepository::create(
                &service.pool,
                &user_event.user_id,
                &user_event.public_key,
                user_event.tx_type.clone(),
                user_event.metadata.as_ref(),
                user_event.timestamp,
            )
            .await
            {
                Ok(_) => {
                    processed_count += 1;
                    info!(
                        "User event inserted successfully: user_id={}, tx_type={:?}",
                        user_event.user_id,
                        user_event.tx_type
                    );
                }
                Err(e) => {
                    error!("Failed to insert user event: {:?}", e);
                    continue;
                }
            }
        }
    }

    // NEW: Broadcast to unified WebSocket connections
    if let Some(ref worker_events) = payload.worker_events {
        info!(
            "Broadcasting {} worker events to unified WebSocket subscribers",
            worker_events.len()
        );
        for worker_event in worker_events {
            service
                .unified_websocket_manager
                .broadcast_worker_event(worker_event)
                .await;

            // BACKWARD COMPATIBILITY: Also broadcast to legacy connections during migration,
            // will be removed later
            service
                .worker_event_manager
                .broadcast_event(worker_event)
                .await;
        }
    }

    if let Some(ref user_events) = payload.user_events {
        info!(
            "Broadcasting {} user events to unified WebSocket subscribers",
            user_events.len()
        );
        for user_event in user_events {
            service
                .unified_websocket_manager
                .broadcast_user_event(user_event)
                .await;

            // BACKWARD COMPATIBILITY: Also broadcast to legacy connections during migration
            // will be removed later
            service
                .user_event_manager
                .broadcast_event(user_event)
                .await;
        }
    }

    info!(
        "Telemetry processing completed successfully: processed {} events",
        processed_count
    );

    Ok(Json(TelemetryResponse {
        success: true,
        processed_count,
        service: auth.claims.sub,
    }))
}


async fn report_checkpoint_leafs_handler(
    State(service): State<ApiService>,
    Extension(auth): Extension<AuthExtension>,
    Json(payload): Json<CheckpointLeavesRequest>,
) -> Result<Json<CheckpointLeavesResponse>, StatusCode> {
    info!(
        "received leaves from {},  leaves len {}",
        auth.claims.sub,
        payload.leaves.len()
    );

    let (min_id, max_id) = match get_checkpoint_range(&payload.leaves) {
        Some(range) => range,
        None => {
            error!("No checkpoints found");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    info!("🍃 Checkpoint ID range: {} - {}", min_id, max_id);
    let mut processed_count = 0;

    for payload in payload.leaves {
        let leaf = CheckpointLeafStat {
            checkpoint_id: payload.checkpoint_id,
            fees_collected: payload.fees_collected,
            user_ops_processed: payload.user_ops_processed,
            total_transactions: payload.total_transactions,
            slots_modified: payload.slots_modified,
            metadata: payload.metadata,
            timestamp: payload.timestamp,
        };

        match CheckpointStatsRepository::create(&service.pool, &leaf).await {
            Ok(_) => {
                processed_count += 1;
            }
            Err(e) => {
                error!(
                    "❌ Failed to create checkpoint leaf at {}: {}",
                    leaf.checkpoint_id,
                    e
                );
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    info!(
        "✅ Successfully reported {} checkpoint leafs ({}~{}) from '{}'",
        processed_count,
        min_id,
        max_id,
        auth.claims.sub
    );
    Ok(Json(CheckpointLeavesResponse {
        success: true,
        processed_count,
        message: format!("Successfully reported {} checkpoint leafs (checkpoints {} to {})", processed_count, min_id, max_id),
    }))
}

fn get_checkpoint_range(stats: &[CheckpointLeafStat]) -> Option<(i64, i64)> {
    stats
        .iter()
        .map(|s| s.checkpoint_id)
        .min()
        .zip(stats.iter().map(|s| s.checkpoint_id).max())
}