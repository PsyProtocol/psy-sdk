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
        tracing::info!("Adding WebSocket connection: {}", connection_id);
        tracing::info!("Connection details: {:#?}", connection);
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), connection);
        tracing::info!("Total active connections: {}", connections.len());
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        tracing::info!("Removing WebSocket connection: {}", connection_id);
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
        tracing::info!("Total active connections: {}", connections.len());
    }

    pub async fn update_filters(&self, connection_id: &ConnectionId, filters: SubscriptionFilters) {
        tracing::info!("Updating filters for connection: {}", connection_id);
        tracing::info!("New filters: {:?}", filters);
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.filters = filters;
            tracing::info!(
                "Filters updated successfully for connection: {}",
                connection_id
            );
        } else {
            tracing::warn!("Connection not found for filter update: {}", connection_id);
        }
    }

    pub async fn broadcast_event(&self, event: &WebSocketEvent) {
        tracing::info!("Broadcasting WebSocket event: {:?}", event.event_type);
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut sent_count = 0;

        for (connection_id, connection) in connections.iter() {
            if self.should_send_event(connection, event) {
                let message =
                    Message::Text(serde_json::to_string(event).unwrap_or_default().into());
                match connection.sender.send(message) {
                    Ok(_) => {
                        sent_count += 1;
                        tracing::info!("Event sent to connection: {}", connection_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to send event to connection {}: {}",
                            connection_id,
                            e
                        );
                    }
                }
            } else {
                tracing::trace!("Event filtered out for connection: {}", connection_id);
            }
        }

        tracing::info!(
            "Broadcast completed: sent to {}/{} connections for event type {:?}",
            sent_count,
            total_connections,
            event.event_type
        );
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
    tracing::info!("New WebSocket connection established: {}", connection_id);

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
    let outgoing_connection_id = connection_id.clone();
    tokio::spawn(async move {
        tracing::info!(
            "Started outgoing message handler for connection: {}",
            outgoing_connection_id
        );
        while let Some(message) = rx.recv().await {
            tracing::info!("Sending message to connection: {}", outgoing_connection_id);
            if ws_sender.send(message).await.is_err() {
                tracing::warn!(
                    "Failed to send message to connection: {}",
                    outgoing_connection_id
                );
                break;
            }
        }
        tracing::info!(
            "Outgoing message handler ended for connection: {}",
            outgoing_connection_id
        );
    });

    // Handle incoming messages
    let incoming_task = tokio::spawn(async move {
        tracing::info!(
            "Started incoming message handler for connection: {}",
            connection_id_clone
        );
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    tracing::info!(
                        "Received text message from connection {}: {}",
                        connection_id_clone,
                        text
                    );
                    match serde_json::from_str::<UpdateConfigurationMessage>(&text) {
                        Ok(config) => {
                            manager_clone
                                .update_filters(&connection_id_clone, config.filters)
                                .await;
                            tracing::info!(
                                "Updated filters for connection {}",
                                connection_id_clone
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse filter configuration from connection {}: {}",
                                connection_id_clone,
                                e
                            );
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Closing connection {}", connection_id_clone);
                    break;
                }
                Ok(Message::Ping(_)) => {
                    tracing::info!("Received ping from connection {}", connection_id_clone);
                }
                Ok(Message::Pong(_)) => {
                    tracing::info!("Received pong from connection {}", connection_id_clone);
                }
                Err(e) => {
                    tracing::error!(
                        "WebSocket error for connection {}: {}",
                        connection_id_clone,
                        e
                    );
                    break;
                }
                _ => {
                    tracing::warn!(
                        "Received other message type from connection {}",
                        connection_id_clone
                    );
                }
            }
        }

        // Remove connection when done
        manager_clone.remove_connection(&connection_id_clone).await;
        tracing::info!("Removed connection {}", connection_id_clone);
    });

    // Wait for incoming task to complete
    let _ = incoming_task.await;
}
