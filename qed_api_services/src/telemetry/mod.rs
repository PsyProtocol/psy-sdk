use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};

use crate::{models::*, services::ApiService};

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
    State(_service): State<ApiService>,
    Json(payload): Json<TelemetryPayload>,
) -> Result<Json<TelemetryResponse>, StatusCode> {
    let mut processed_count = 0;

    // TODO: Process worker events
    if let Some(worker_events) = &payload.worker_events {
        processed_count += worker_events.len();
        // TODO: Insert into database
    }

    // TODO: Process user events
    if let Some(user_events) = &payload.user_events {
        processed_count += user_events.len();
        // TODO: Insert into database
    }

    // TODO: Push to WebSocket subscribers

    Ok(Json(TelemetryResponse {
        success: true,
        processed_count,
    }))
}