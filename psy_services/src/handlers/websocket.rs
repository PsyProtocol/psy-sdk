pub mod tps;
pub mod user_event;
pub mod worker_event;

use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
pub use user_event::{UserEventConnection, UserEventFilters, UserEventManager};
pub use worker_event::{WorkerEventConnection, WorkerEventFilters, WorkerEventManager};

use crate::services::ApiService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    WorkerEvent,
    UserEvent,
    TpsUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn create_websocket_router(api_service: ApiService) -> Router {
    Router::new()
        .route("/ws/user_event", get(user_event::user_event_websocket_handler))
        .route("/ws/worker_event", get(worker_event::worker_event_websocket_handler))
        .route("/ws/tps", get(tps::websocket_tps_handler))
        .with_state(api_service)
}
