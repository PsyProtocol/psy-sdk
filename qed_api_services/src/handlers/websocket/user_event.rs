use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};

use crate::{models::*, services::ApiService};

pub type ConnectionId = String;

#[derive(Debug, Clone)]
pub struct UserEventManager {
    pub connections: Arc<RwLock<HashMap<ConnectionId, UserEventConnection>>>,
}

#[derive(Debug, Clone)]
pub struct UserEventConnection {
    pub user_id: Option<String>,
    pub filters: UserEventFilters,
    pub sender: mpsc::UnboundedSender<Message>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserEventFilters {
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserEventConfigurationMessage {
    pub filters: UserEventFilters,
}

use super::{EventType, WebSocketEvent};

impl UserEventManager {
    pub async fn add_connection(
        &self,
        connection_id: ConnectionId,
        connection: UserEventConnection,
    ) {
        tracing::info!("Adding user event WebSocket connection: {}", connection_id);
        tracing::info!("Connection details: {:#?}", connection);
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), connection);
        tracing::info!("Total active user event connections: {}", connections.len());
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        tracing::info!(
            "Removing user event WebSocket connection: {}",
            connection_id
        );
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
        tracing::info!("Total active user event connections: {}", connections.len());
    }

    pub async fn update_filters(&self, connection_id: &ConnectionId, filters: UserEventFilters) {
        tracing::info!(
            "Updating user event filters for connection: {}",
            connection_id
        );
        tracing::info!("New filters: {:?}", filters);
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.filters = filters;
            tracing::info!(
                "User event filters updated successfully for connection: {}",
                connection_id
            );
        } else {
            tracing::warn!(
                "User event connection not found for filter update: {}",
                connection_id
            );
        }
    }

    pub async fn broadcast_event(&self, event: &UserEvent) {
        tracing::info!("Broadcasting user event to connections");
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut sent_count = 0;

        let websocket_event = WebSocketEvent {
            event_type: EventType::UserEvent,
            data: serde_json::to_value(event).unwrap_or_default(),
            timestamp: event.timestamp,
        };

        for (connection_id, connection) in connections.iter() {
            if self.should_send_event(connection, event) {
                let message = Message::Text(
                    serde_json::to_string(&websocket_event)
                        .unwrap_or_default()
                        .into(),
                );
                match connection.sender.send(message) {
                    Ok(_) => {
                        sent_count += 1;
                        tracing::info!("User event sent to connection: {}", connection_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to send user event to connection {}: {}",
                            connection_id,
                            e
                        );
                    }
                }
            } else {
                tracing::trace!("User event filtered out for connection: {}", connection_id);
            }
        }

        tracing::info!(
            "User event broadcast completed: sent to {}/{} connections",
            sent_count,
            total_connections
        );
    }

    fn should_send_event(&self, connection: &UserEventConnection, event: &UserEvent) -> bool {
        if let Some(filter_user_id) = &connection.filters.user_id {
            return filter_user_id == &event.user_id;
        }
        true
    }
}

pub async fn user_event_websocket_handler(
    ws: WebSocketUpgrade,
    State(service): State<ApiService>,
) -> Response {
    ws.on_upgrade(move |socket| handle_user_event_socket(socket, service))
}

async fn handle_user_event_socket(socket: WebSocket, service: ApiService) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let connection_id = uuid::Uuid::new_v4().to_string();
    tracing::info!(
        "New user event WebSocket connection established: {}",
        connection_id
    );

    let connection_id_clone = connection_id.clone();
    let manager = service.user_event_manager.clone();
    let manager_clone = service.user_event_manager.clone();

    // Add connection to manager
    let connection = UserEventConnection {
        user_id: None,
        filters: UserEventFilters::default(),
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
            "Started user event outgoing message handler for connection: {}",
            outgoing_connection_id
        );
        while let Some(message) = rx.recv().await {
            tracing::info!(
                "Sending user event message to connection: {}",
                outgoing_connection_id
            );
            if ws_sender.send(message).await.is_err() {
                tracing::warn!(
                    "Failed to send user event message to connection: {}",
                    outgoing_connection_id
                );
                break;
            }
        }
        tracing::info!(
            "User event outgoing message handler ended for connection: {}",
            outgoing_connection_id
        );
    });

    // Handle incoming messages
    let incoming_task = tokio::spawn(async move {
        tracing::info!(
            "Started user event incoming message handler for connection: {}",
            connection_id_clone
        );
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    tracing::info!(
                        "Received user event text message from connection {}: {}",
                        connection_id_clone,
                        text
                    );
                    match serde_json::from_str::<UpdateUserEventConfigurationMessage>(&text) {
                        Ok(config) => {
                            manager_clone
                                .update_filters(&connection_id_clone, config.filters)
                                .await;
                            tracing::info!(
                                "Updated user event filters for connection {}",
                                connection_id_clone
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse user event filter configuration from connection {}: {}", connection_id_clone, e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Closing user event connection {}", connection_id_clone);
                    break;
                }
                Ok(Message::Ping(_)) => {
                    tracing::info!(
                        "Received ping from user event connection {}",
                        connection_id_clone
                    );
                }
                Ok(Message::Pong(_)) => {
                    tracing::info!(
                        "Received pong from user event connection {}",
                        connection_id_clone
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "User event WebSocket error for connection {}: {}",
                        connection_id_clone,
                        e
                    );
                    break;
                }
                _ => {
                    tracing::warn!(
                        "Received other message type from user event connection {}",
                        connection_id_clone
                    );
                }
            }
        }

        // Remove connection when done
        manager_clone.remove_connection(&connection_id_clone).await;
        tracing::info!("Removed user event connection {}", connection_id_clone);
    });

    // Wait for incoming task to complete
    let _ = incoming_task.await;
}
