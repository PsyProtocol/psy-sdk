use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};

use crate::{
    models::*,
    repositories::*,
    services::ApiService,
    websocket::{EventType, WebSocketEvent},
};

pub fn create_telemetry_router(api_service: ApiService) -> Router {
    Router::new()
        .route("/telemetry/events", post(receive_events_handler))
        .with_state(api_service)
}

#[derive(Debug, Deserialize)]
pub struct TelemetryPayload {
    pub worker_events: Option<Vec<WorkerEvent>>,
    pub user_events: Option<Vec<UserEvent>>,
}

#[derive(Debug, Serialize)]
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

    // Push events to WebSocket subscribers
    if let Some(ref worker_events) = payload.worker_events {
        tracing::info!(
            "Broadcasting {} worker events to WebSocket subscribers",
            worker_events.len()
        );
        for worker_event in worker_events {
            let ws_event = WebSocketEvent {
                event_type: EventType::WorkerEvent,
                data: serde_json::to_value(worker_event).unwrap_or_default(),
                timestamp: chrono::Utc::now(),
            };
            service.websocket_manager.broadcast_event(&ws_event).await;
        }
    }

    if let Some(ref user_events) = payload.user_events {
        tracing::info!(
            "Broadcasting {} user events to WebSocket subscribers",
            user_events.len()
        );
        for user_event in user_events {
            let ws_event = WebSocketEvent {
                event_type: EventType::UserEvent,
                data: serde_json::to_value(user_event).unwrap_or_default(),
                timestamp: chrono::Utc::now(),
            };
            service.websocket_manager.broadcast_event(&ws_event).await;
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
