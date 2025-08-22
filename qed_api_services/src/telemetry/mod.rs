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
    let mut processed_count = 0;

    // Process worker events
    if let Some(ref worker_events) = payload.worker_events {
        for worker_event in worker_events {
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
                Ok(_) => processed_count += 1,
                Err(e) => {
                    tracing::error!("Failed to insert worker event: {:?}", e);
                    continue;
                }
            }
        }
    }

    // Process user events
    if let Some(ref user_events) = payload.user_events {
        for user_event in user_events {
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
                Ok(_) => processed_count += 1,
                Err(e) => {
                    tracing::error!("Failed to insert user event: {:?}", e);
                    continue;
                }
            }
        }
    }

    // Push events to WebSocket subscribers
    if let Some(ref worker_events) = payload.worker_events {
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
        for user_event in user_events {
            let ws_event = WebSocketEvent {
                event_type: EventType::UserEvent,
                data: serde_json::to_value(user_event).unwrap_or_default(),
                timestamp: chrono::Utc::now(),
            };
            service.websocket_manager.broadcast_event(&ws_event).await;
        }
    }

    Ok(Json(TelemetryResponse {
        success: true,
        processed_count,
    }))
}
