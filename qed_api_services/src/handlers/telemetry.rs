use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{models::*, repositories::*, services::ApiService};
use crate::repositories::checkpoint_state::{CheckpointStatsRepository, WorkerJobEventRepository};

pub fn create_telemetry_router(api_service: ApiService) -> Router {
    Router::new()
        // Legacy event reporting
        .route("/telemetry/events", post(receive_events_handler))

        // Checkpoint Stats Reporting
        .route("/telemetry/checkpoint/stats", post(report_checkpoint_stats_handler))

        // Worker Job Events Reporting
        .route("/telemetry/checkpoint/job-events", post(report_worker_job_events_handler))

        .with_state(api_service)
}

// ============================================================================
// LEGACY EVENT REPORTING
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryPayload {
    pub worker_events: Option<Vec<WorkerEvent>>,
    pub user_events: Option<Vec<UserEvent>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryResponse {
    pub success: bool,
    pub processed_count: usize,
}

async fn receive_events_handler(
    State(service): State<ApiService>,
    Json(payload): Json<TelemetryPayload>,
) -> Result<Json<TelemetryResponse>, StatusCode> {
    tracing::info!(
        "Telemetry events received: worker_events={}, user_events={}",
        payload.worker_events.as_ref().map(|v| v.len()).unwrap_or(0),
        payload.user_events.as_ref().map(|v| v.len()).unwrap_or(0)
    );
    tracing::info!("Telemetry payload: {:?}", payload);

    let mut processed_count = 0;

    // Process worker events
    if let Some(ref worker_events) = payload.worker_events {
        tracing::info!("Processing {} worker events", worker_events.len());
        for (index, worker_event) in worker_events.iter().enumerate() {
            tracing::info!(
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
                    tracing::info!(
                        "Worker event inserted successfully: job_id={:?}",
                        worker_event.job_id
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to insert worker event: {:?}", e);
                    continue;
                }
            }
        }
    }

    // Process user events
    if let Some(ref user_events) = payload.user_events {
        tracing::info!("Processing {} user events", user_events.len());
        for (index, user_event) in user_events.iter().enumerate() {
            tracing::info!(
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
                    tracing::info!(
                        "User event inserted successfully: user_id={}, tx_type={:?}",
                        user_event.user_id,
                        user_event.tx_type
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to insert user event: {:?}", e);
                    continue;
                }
            }
        }
    }

    if let Some(ref worker_events) = payload.worker_events {
        tracing::info!(
            "Broadcasting {} worker events to WebSocket subscribers",
            worker_events.len()
        );
        for worker_event in worker_events {
            service
                .worker_event_manager
                .broadcast_event(worker_event)
                .await;
        }
    }

    if let Some(ref user_events) = payload.user_events {
        tracing::info!(
            "Broadcasting {} user events to WebSocket subscribers",
            user_events.len()
        );
        for user_event in user_events {
            service.user_event_manager.broadcast_event(user_event).await;
        }
    }

    tracing::info!(
        "Telemetry processing completed successfully: processed {} events",
        processed_count
    );

    Ok(Json(TelemetryResponse {
        success: true,
        processed_count,
    }))
}

async fn report_checkpoint_stats_handler(
    State(service): State<ApiService>,
    Json(payload): Json<CheckpointStatsRequest>,
) -> Result<Json<CheckpointStatsResponse>, StatusCode> {
    tracing::info!(
        "📊 Telemetry: Received checkpoint stats for checkpoint {}",
        payload.checkpoint_id
    );

    let create_stats = CreateCheckpointStats {
        checkpoint_id: payload.checkpoint_id,
        fees_collected: payload.fees_collected,
        user_ops_processed: payload.user_ops_processed,
        total_transactions: payload.total_transactions,
        slots_modified: payload.slots_modified,
        metadata: payload.metadata,
        timestamp: payload.timestamp,
    };

    match CheckpointStatsRepository::create(&service.pool, &create_stats).await {
        Ok(stats) => {
            tracing::info!(
                "✅ Successfully reported checkpoint stats for checkpoint {}",
                stats.checkpoint_id
            );
            Ok(Json(CheckpointStatsResponse {
                success: true,
                checkpoint_id: stats.checkpoint_id,
                message: format!("Checkpoint stats reported successfully"),
            }))
        }
        Err(e) => {
            tracing::error!("❌ Failed to report checkpoint stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


async fn report_worker_job_events_handler(
    State(service): State<ApiService>,
    Json(payload): Json<Vec<WorkerJobEventRequest>>,
) -> Result<Json<WorkerJobEventsResponse>, StatusCode> {
    if payload.is_empty() {
        return Ok(Json(WorkerJobEventsResponse {
            success: true,
            events_reported: 0,
            checkpoint_id: 0,
            message: "No events to report".to_string(),
        }));
    }

    let checkpoint_id = payload[0].checkpoint_id;
    tracing::info!(
        "🔧 Telemetry: Received {} worker job events for checkpoint {}",
        payload.len(),
        checkpoint_id
    );

    let create_events: Vec<CreateWorkerJobEvent> = payload
        .into_iter()
        .map(|e| CreateWorkerJobEvent {
            worker_public_key: e.worker_public_key,
            checkpoint_id: e.checkpoint_id,
            job_id: e.job_id,
            topic: e.topic,
            circuit_type: e.circuit_type,
            duration: e.duration,
            status: e.status,
            metadata: e.metadata,
            timestamp: e.timestamp,
        })
        .collect();

    match WorkerJobEventRepository::create_batch(&service.pool, &create_events).await {
        Ok(events) => {
            tracing::info!(
                "✅ Successfully reported {} worker job events for checkpoint {}",
                events.len(),
                checkpoint_id
            );
            Ok(Json(WorkerJobEventsResponse {
                success: true,
                events_reported: events.len(),
                checkpoint_id,
                message: format!("Successfully reported {} job events", events.len()),
            }))
        }
        Err(e) => {
            tracing::error!("❌ Failed to report worker job events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}