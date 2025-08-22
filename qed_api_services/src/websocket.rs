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
use tokio::sync::{mpsc, RwLock};

use crate::{models::*, services::ApiService};

pub type ConnectionId = String;

#[derive(Debug, Clone)]
pub struct WebSocketManager {
    pub connections: Arc<RwLock<HashMap<ConnectionId, WebSocketConnection>>>,
}

#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub user_id: Option<String>,
    pub filters: SubscriptionFilters,
    pub sender: mpsc::UnboundedSender<Message>,
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

#[derive(Debug, Serialize, Clone)]
pub enum EventType {
    WorkerEvent,
    UserEvent,
}

#[derive(Debug, Serialize)]
pub struct WebSocketEvent {
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl WebSocketManager {
    pub async fn add_connection(
        &self,
        connection_id: ConnectionId,
        connection: WebSocketConnection,
    ) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id, connection);
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
    }

    pub async fn update_filters(&self, connection_id: &ConnectionId, filters: SubscriptionFilters) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.filters = filters;
        }
    }

    pub async fn broadcast_event(&self, event: &WebSocketEvent) {
        let connections = self.connections.read().await;
        for (_, connection) in connections.iter() {
            if self.should_send_event(connection, event) {
                let message =
                    Message::Text(serde_json::to_string(event).unwrap_or_default().into());
                let _ = connection.sender.send(message);
            }
        }
    }

    fn should_send_event(&self, connection: &WebSocketConnection, event: &WebSocketEvent) -> bool {
        // Basic filtering logic - can be expanded based on requirements
        match event.event_type {
            EventType::WorkerEvent => {
                // Filter by realm_id if specified
                if let Some(realm_ids) = &connection.filters.realm_ids {
                    if let Ok(worker_event) =
                        serde_json::from_value::<WorkerEvent>(event.data.clone())
                    {
                        if let Some(realm_id) = worker_event.realm_id {
                            return realm_ids.contains(&realm_id.to_string());
                        }
                    }
                }
                true
            }
            EventType::UserEvent => {
                // Filter by user_id if specified
                if let Some(user_ids) = &connection.filters.user_ids {
                    if let Ok(user_event) = serde_json::from_value::<UserEvent>(event.data.clone())
                    {
                        return user_ids.contains(&user_event.user_id);
                    }
                }
                true
            }
        }
    }
}

pub fn create_websocket_router(api_service: ApiService) -> Router {
    Router::new()
        .route("/ws/subscribe", get(websocket_handler))
        .with_state(api_service)
}

async fn websocket_handler(ws: WebSocketUpgrade, State(service): State<ApiService>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, service))
}

async fn handle_socket(socket: WebSocket, service: ApiService) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let connection_id = uuid::Uuid::new_v4().to_string();
    let connection_id_clone = connection_id.clone();
    let manager = service.websocket_manager.clone();
    let manager_clone = service.websocket_manager.clone();

    // Add connection to manager
    let connection = WebSocketConnection {
        user_id: None,
        filters: SubscriptionFilters::default(),
        sender: tx,
    };

    manager
        .add_connection(connection_id.clone(), connection)
        .await;

    // Spawn task to handle outgoing messages
    let mut ws_sender = ws_sender;
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(config) = serde_json::from_str::<UpdateConfigurationMessage>(&text) {
                        manager_clone
                            .update_filters(&connection_id_clone, config.filters)
                            .await;
                        tracing::info!("Updated filters for connection {}", connection_id_clone);
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Closing connection {}", connection_id_clone);
                    break;
                }
                _ => {}
            }
        }

        // Remove connection when done
        manager_clone.remove_connection(&connection_id_clone).await;
        tracing::info!("Removed connection {}", connection_id_clone);
    });

    // Wait for incoming task to complete
    let _ = incoming_task.await;
}
