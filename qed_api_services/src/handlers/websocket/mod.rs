
pub mod tps;
pub mod user_event;
pub mod worker_event;
pub mod unified;

pub use user_event::{UserEventConnection, UserEventFilters, UserEventManager};
pub use worker_event::{WorkerEventConnection, WorkerEventFilters, WorkerEventManager};
pub use unified::{UnifiedWebSocketManager, unified_websocket_handler};  

use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};

use crate::services::ApiService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    Worker,
    User,
    Tps,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn create_websocket_router(api_service: ApiService) -> Router {
    Router::new()
        // NEW: Single unified endpoint
        .route("/ws", get(unified::unified_websocket_handler))

        // DEPRECATED: Keep old endpoints for backward compatibility
        // These can be removed after clients migrate to the unified endpoint
        .route(
            "/ws/user_event",
            get(user_event::user_event_websocket_handler),
        )
        .route(
            "/ws/worker_event",
            get(worker_event::worker_event_websocket_handler),
        )
        .route("/ws/tps", get(tps::websocket_tps_handler))
        .with_state(api_service)
}