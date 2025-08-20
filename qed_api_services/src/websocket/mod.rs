use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::services::ApiService;

pub type ConnectionId = String;

#[derive(Debug, Clone)]
pub struct WebSocketManager {
    pub connections: Arc<RwLock<HashMap<ConnectionId, WebSocketConnection>>>,
}

#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub user_id: Option<String>,
    pub filters: SubscriptionFilters,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SubscriptionFilters {
    pub user_ids: Option<HashSet<String>>,
    pub realm_ids: Option<HashSet<String>>,
    pub event_types: Option<HashSet<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigurationMessage {
    pub filters: SubscriptionFilters,
}

pub fn create_websocket_router(api_service: ApiService) -> Router {
    Router::new()
        .route("/ws/subscribe", get(websocket_handler))
        .with_state(api_service)
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(_service): State<ApiService>,
) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    
    let connection_id = uuid::Uuid::new_v4().to_string();
    
    // TODO: Add connection to manager
    // TODO: Handle incoming configuration messages
    // TODO: Send events based on filters
    
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // TODO: Parse configuration update message
                    if let Ok(_config) = serde_json::from_str::<UpdateConfigurationMessage>(&text) {
                        // TODO: Update connection filters
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                _ => {}
            }
        }
        // TODO: Remove connection from manager
    });
    
    // Keep connection alive with ping/pong
    loop {
        if sender.send(Message::Ping(vec![].into())).await.is_err() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}