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
pub struct WorkerEventManager {
    pub connections: Arc<RwLock<HashMap<ConnectionId, WorkerEventConnection>>>,
}

#[derive(Debug, Clone)]
pub struct WorkerEventConnection {
    pub user_id: Option<String>,
    pub filters: WorkerEventFilters,
    pub sender: mpsc::UnboundedSender<Message>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkerEventFilters {
    pub realm_id: Option<String>,
    pub worker_pubkey: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkerEventConfigurationMessage {
    pub filters: WorkerEventFilters,
}

use super::{EventType, WebSocketEvent};

impl WorkerEventManager {
    pub async fn add_connection(
        &self,
        connection_id: ConnectionId,
        connection: WorkerEventConnection,
    ) {
        tracing::info!(
            "Adding worker event WebSocket connection: {}",
            connection_id
        );
        tracing::info!("Connection details: {:#?}", connection);
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), connection);
        tracing::info!(
            "Total active worker event connections: {}",
            connections.len()
        );
    }

    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        tracing::info!(
            "Removing worker event WebSocket connection: {}",
            connection_id
        );
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
        tracing::info!(
            "Total active worker event connections: {}",
            connections.len()
        );
    }

    pub async fn update_filters(&self, connection_id: &ConnectionId, filters: WorkerEventFilters) {
        tracing::info!(
            "Updating worker event filters for connection: {}",
            connection_id
        );
        tracing::info!("New filters: {:?}", filters);
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.filters = filters;
            tracing::info!(
                "Worker event filters updated successfully for connection: {}",
                connection_id
            );
        } else {
            tracing::warn!(
                "Worker event connection not found for filter update: {}",
                connection_id
            );
        }
    }

    pub async fn broadcast_event(&self, event: &WorkerEvent) {
        tracing::info!("Broadcasting worker event to connections");
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut sent_count = 0;

        let websocket_event = WebSocketEvent {
            event_type: EventType::WorkerEvent,
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
                        tracing::info!("Worker event sent to connection: {}", connection_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to send worker event to connection {}: {}",
                            connection_id,
                            e
                        );
                    }
                }
            } else {
                tracing::trace!(
                    "Worker event filtered out for connection: {}",
                    connection_id
                );
            }
        }

        tracing::info!(
            "Worker event broadcast completed: sent to {}/{} connections",
            sent_count,
            total_connections
        );
    }

    fn should_send_event(&self, connection: &WorkerEventConnection, event: &WorkerEvent) -> bool {
        // Filter by realm_id if specified
        if let Some(filter_realm_id) = &connection.filters.realm_id {
            if let Some(event_realm_id) = event.realm_id {
                if filter_realm_id != &event_realm_id.to_string() {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filter by worker public key if specified
        if let Some(filter_worker_pubkeys) = &connection.filters.worker_pubkey {
            if let Some(event_worker_pubkey) = &event.public_key {
                return filter_worker_pubkeys.contains(event_worker_pubkey);
            } else {
                return false;
            }
        }

        true
    }
}

pub async fn worker_event_websocket_handler(
    ws: WebSocketUpgrade,
    State(service): State<ApiService>,
) -> Response {
    ws.on_upgrade(move |socket| handle_worker_event_socket(socket, service))
}

async fn handle_worker_event_socket(socket: WebSocket, service: ApiService) {
    let (ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let connection_id = uuid::Uuid::new_v4().to_string();
    tracing::info!(
        "New worker event WebSocket connection established: {}",
        connection_id
    );

    let connection_id_clone = connection_id.clone();
    let manager = service.worker_event_manager.clone();
    let manager_clone = service.worker_event_manager.clone();

    // Add connection to manager
    let connection = WorkerEventConnection {
        user_id: None,
        filters: WorkerEventFilters::default(),
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
            "Started worker event outgoing message handler for connection: {}",
            outgoing_connection_id
        );
        while let Some(message) = rx.recv().await {
            tracing::info!(
                "Sending worker event message to connection: {}",
                outgoing_connection_id
            );
            if ws_sender.send(message).await.is_err() {
                tracing::warn!(
                    "Failed to send worker event message to connection: {}",
                    outgoing_connection_id
                );
                break;
            }
        }
        tracing::info!(
            "Worker event outgoing message handler ended for connection: {}",
            outgoing_connection_id
        );
    });

    // Handle incoming messages
    let incoming_task = tokio::spawn(async move {
        tracing::info!(
            "Started worker event incoming message handler for connection: {}",
            connection_id_clone
        );
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    tracing::info!(
                        "Received worker event text message from connection {}: {}",
                        connection_id_clone,
                        text
                    );
                    match serde_json::from_str::<UpdateWorkerEventConfigurationMessage>(&text) {
                        Ok(config) => {
                            manager_clone
                                .update_filters(&connection_id_clone, config.filters)
                                .await;
                            tracing::info!(
                                "Updated worker event filters for connection {}",
                                connection_id_clone
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse worker event filter configuration from connection {}: {}", connection_id_clone, e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Closing worker event connection {}", connection_id_clone);
                    break;
                }
                Ok(Message::Ping(_)) => {
                    tracing::info!(
                        "Received ping from worker event connection {}",
                        connection_id_clone
                    );
                }
                Ok(Message::Pong(_)) => {
                    tracing::info!(
                        "Received pong from worker event connection {}",
                        connection_id_clone
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Worker event WebSocket error for connection {}: {}",
                        connection_id_clone,
                        e
                    );
                    break;
                }
                _ => {
                    tracing::warn!(
                        "Received other message type from worker event connection {}",
                        connection_id_clone
                    );
                }
            }
        }

        // Remove connection when done
        manager_clone.remove_connection(&connection_id_clone).await;
        tracing::info!("Removed worker event connection {}", connection_id_clone);
    });

    // Wait for incoming task to complete
    let _ = incoming_task.await;
}
