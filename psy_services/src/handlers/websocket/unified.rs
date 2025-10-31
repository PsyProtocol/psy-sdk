use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    Json,
};
use futures::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    models::{TpsData, UserEvent, WorkerEvent},
    services::ApiService,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe { channels: Vec<Channel> },
    Unsubscribe { channels: Vec<Channel> },
    UpdateFilters { filters: ChannelFilters },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Event {
        channel: Channel,
        data: serde_json::Value,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Subscribed {
        channels: Vec<Channel>,
    },
    Unsubscribed {
        channels: Vec<Channel>,
    },
    Error {
        code: String,
        message: String,
    },
    Ping, // Server sends ping to client
    Pong, // Server responds to client ping
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    WorkerEvents,
    UserEvents,
    Tps,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelFilters {
    // Worker event filters
    pub worker_realm_id: Option<i64>,
    pub worker_public_key: Option<String>,
    pub worker_status: Option<String>,

    // User event filters
    pub user_id: Option<String>,
    pub user_public_key: Option<String>,
    pub tx_type: Option<String>,

    // TPS filters (extensible)
    pub tps_interval: Option<u32>, // Update interval in seconds
}

pub struct UnifiedWebSocketConnection {
    pub id: Uuid,
    pub sender: mpsc::UnboundedSender<ServerMessage>,
    pub subscriptions: HashSet<Channel>,
    pub filters: ChannelFilters,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_pong_at: Arc<RwLock<chrono::DateTime<chrono::Utc>>>, // NEW
}

#[derive(Clone)]
pub struct UnifiedWebSocketManager {
    connections: Arc<RwLock<HashMap<Uuid, UnifiedWebSocketConnection>>>,
}

impl UnifiedWebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, conn: UnifiedWebSocketConnection) {
        let mut connections = self.connections.write().await;
        info!("Added unified websocket connection: {}", &conn.id);
        connections.insert(conn.id, conn);
    }

    pub async fn remove_connection(&self, id: Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(&id);
        info!("Removed unified websocket connection: {}", id);
    }

    /// Update last pong time for a connection
    pub async fn update_pong_time(&self, id: Uuid) {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(&id) {
            let mut last_pong = conn.last_pong_at.write().await;
            *last_pong = chrono::Utc::now();
            debug!("Updated pong time for connection {}", id);
        }
    }

    /// Check if connection is alive based on last pong time
    /// Returns true if connection should be kept, false if it should be closed
    async fn is_connection_alive(&self, conn: &UnifiedWebSocketConnection) -> bool {
        const PONG_TIMEOUT_SECS: i64 = 30;

        let last_pong = conn.last_pong_at.read().await;
        let elapsed = chrono::Utc::now().signed_duration_since(*last_pong);

        if elapsed.num_seconds() > PONG_TIMEOUT_SECS {
            warn!("Connection {} timed out: no pong received in {} seconds", conn.id, elapsed.num_seconds());
            false
        } else {
            true
        }
    }

    pub async fn update_subscriptions(&self, id: Uuid, channels: Vec<Channel>, subscribe: bool) -> Result<Vec<Channel>, String> {
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(&id) {
            let mut changed = Vec::new();

            for channel in channels {
                if subscribe {
                    if conn.subscriptions.insert(channel) {
                        changed.push(channel);
                    }
                } else {
                    if conn.subscriptions.remove(&channel) {
                        changed.push(channel);
                    }
                }
            }

            Ok(changed)
        } else {
            Err("Connection not found".to_string())
        }
    }

    pub async fn update_filters(&self, id: Uuid, filters: ChannelFilters) -> Result<(), String> {
        let mut connections = self.connections.write().await;

        if let Some(conn) = connections.get_mut(&id) {
            conn.filters = filters;
            Ok(())
        } else {
            Err("Connection not found".to_string())
        }
    }

    // Broadcast a worker event to all subscribed connections
    pub async fn broadcast_worker_event(&self, event: &WorkerEvent) {
        let connections = self.connections.read().await;
        let mut dead_connections = Vec::new();

        for conn in connections.values() {
            // Check if the connection is still alive
            if !self.is_connection_alive(conn).await {
                dead_connections.push(conn.id);
                continue;
            }

            // Check if subscribed to worker events
            if !conn.subscriptions.contains(&Channel::WorkerEvents) {
                continue;
            }

            // Apply filters
            if let Some(realm_id) = conn.filters.worker_realm_id {
                if event.realm_id != Some(realm_id) {
                    continue;
                }
            }

            if let Some(ref public_key) = conn.filters.worker_public_key {
                if event.public_key.as_deref() != Some(public_key.as_str()) {
                    continue;
                }
            }

            if let Some(ref status) = conn.filters.worker_status {
                if format!("{:?}", event.status) != *status {
                    continue;
                }
            }

            // Send the event
            let message = ServerMessage::Event {
                channel: Channel::WorkerEvents,
                data: serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                timestamp: chrono::Utc::now(),
            };

            let _ = conn.sender.send(message);
        }

        // Clean up dead connections
        drop(connections);
        for conn_id in dead_connections {
            self.remove_connection(conn_id).await;
        }
    }

    // Broadcast a user event to all subscribed connections
    pub async fn broadcast_user_event(&self, event: &UserEvent) {
        let connections = self.connections.read().await;
        let mut dead_connections = Vec::new();

        for conn in connections.values() {
            // Check if the connection is still alive
            if !self.is_connection_alive(conn).await {
                dead_connections.push(conn.id);
                continue;
            }

            // Check if subscribed to user events
            if !conn.subscriptions.contains(&Channel::UserEvents) {
                continue;
            }

            // Apply filters
            if let Some(ref user_id) = conn.filters.user_id {
                if event.user_id != *user_id {
                    continue;
                }
            }

            if let Some(ref public_key) = conn.filters.user_public_key {
                if event.public_key != *public_key {
                    continue;
                }
            }

            if let Some(ref tx_type) = conn.filters.tx_type {
                if format!("{:?}", event.tx_type) != *tx_type {
                    continue;
                }
            }

            // Send the event
            let message = ServerMessage::Event {
                channel: Channel::UserEvents,
                data: serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                timestamp: chrono::Utc::now(),
            };

            let _ = conn.sender.send(message);
        }

        // Clean up dead connections
        drop(connections);
        for conn_id in dead_connections {
            self.remove_connection(conn_id).await;
        }
    }

    // Broadcast TPS data to all subscribed connections
    pub async fn broadcast_tps(&self, tps_data: &TpsData) {
        let connections = self.connections.read().await;
        let mut dead_connections = Vec::new();

        for conn in connections.values() {
            // Check if subscribed to TPS updates
            if !conn.subscriptions.contains(&Channel::Tps) {
                continue;
            }

            // Send the TPS update
            let message = ServerMessage::Event {
                channel: Channel::Tps,
                data: serde_json::to_value(tps_data).unwrap_or(serde_json::Value::Null),
                timestamp: chrono::Utc::now(),
            };

            let _ = conn.sender.send(message);
        }

        // Clean up dead connections
        drop(connections);

        for conn_id in dead_connections {
            self.remove_connection(conn_id).await;
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    // Auto-subscribe to channels on connect
    pub channels: Option<String>, // Comma-separated: "worker_events,user_events,tps"

    // Initial filters (optional)
    pub worker_realm_id: Option<i64>,
    pub worker_public_key: Option<String>,
    pub user_id: Option<String>,
}

impl WebSocketQuery {
    fn parse_channels(&self) -> Vec<Channel> {
        self.channels
            .as_ref()
            .map(|s| {
                s.split(',')
                    .filter_map(|c| match c.trim() {
                        "worker_events" => Some(Channel::WorkerEvents),
                        "user_events" => Some(Channel::UserEvents),
                        "tps" => Some(Channel::Tps),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn to_filters(&self) -> ChannelFilters {
        ChannelFilters {
            worker_realm_id: self.worker_realm_id,
            worker_public_key: self.worker_public_key.clone(),
            user_id: self.user_id.clone(),
            ..Default::default()
        }
    }
}

pub async fn unified_websocket_handler(ws: WebSocketUpgrade, State(service): State<ApiService>, Query(query): Query<WebSocketQuery>) -> Response {
    info!("New unified websocket connection request with query: {:?}", query);

    // Parse initial subscriptions and filters from query params
    let initial_channels = query.parse_channels();
    let initial_filters = query.to_filters();

    ws.on_upgrade(move |socket| handle_unified_websocket(socket, service, initial_channels, initial_filters))
}

async fn handle_unified_websocket(socket: WebSocket, service: ApiService, initial_channels: Vec<Channel>, initial_filters: ChannelFilters) {
    let connection_id = Uuid::new_v4();
    let (mut sender, mut receiver) = socket.split();

    // Create channel for sending messages to this connection
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Get the manager from service
    let manager = &service.unified_websocket_manager;

    // Create and add connection
    let now = chrono::Utc::now();
    let mut conn = UnifiedWebSocketConnection {
        id: connection_id,
        sender: tx,
        subscriptions: HashSet::new(),
        filters: initial_filters,
        created_at: now,
        last_pong_at: Arc::new(RwLock::new(now)), // Initialize with current time
    };

    // Apply initial subscriptions
    for channel in initial_channels {
        conn.subscriptions.insert(channel);
    }

    // Send initial subscription confirmation if any
    if !conn.subscriptions.is_empty() {
        let subscribed_channels: Vec<Channel> = conn.subscriptions.iter().copied().collect();
        let _ = conn.sender.send(ServerMessage::Subscribed {
            channels: subscribed_channels.clone(),
        });
        info!("Connection {} auto-subscribed to: {:?}", connection_id, subscribed_channels);
    }

    manager.add_connection(conn).await;

    // Clone manager for use in tasks
    let manager_for_recv = manager.clone();
    let manager_for_ping = manager.clone();

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(Utf8Bytes::from(json))).await.is_err() {
                break;
            }
        }
    });

    // Spawn ping/pong heartbeat task
    let mut ping_task = tokio::spawn(async move {
        use tokio::time::{interval, timeout};

        const PING_INTERVAL: Duration = Duration::from_secs(30);

        let mut ping_interval = interval(PING_INTERVAL);
        ping_interval.tick().await; // Skip first immediate tick

        loop {
            // Wait for next ping interval
            ping_interval.tick().await;

            // Send ping message
            let connections = manager_for_ping.connections.read().await;
            let conn = match connections.get(&connection_id) {
                Some(c) => c,
                None => {
                    debug!("Connection {} not found, stopping ping task", connection_id);
                    break;
                }
            };

            // Send ping through ServerMessage
            if conn.sender.send(ServerMessage::Ping).is_err() {
                warn!("Failed to send ping to connection {}", connection_id);
                break;
            }

            debug!("Sent ping to connection {}", connection_id);
        }

        debug!("Ping task ended for connection {}", connection_id);
    });

    // Handle incoming messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Parse and handle client message
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Pong) => {
                            // Update last pong time
                            manager_for_recv.update_pong_time(connection_id).await;
                            debug!("Received pong from connection {}", connection_id);
                        }
                        Ok(client_msg) => {
                            handle_client_message(&manager_for_recv, connection_id, client_msg).await;
                        }
                        Err(e) => {
                            warn!("Failed to parse client message: {}", e);
                            let error_msg = ServerMessage::Error {
                                code: "PARSE_ERROR".to_string(),
                                message: format!("Invalid message format: {}", e),
                            };

                            // Get connection and send error
                            let connections = manager_for_recv.connections.read().await;
                            if let Some(conn) = connections.get(&connection_id) {
                                let _ = conn.sender.send(error_msg);
                            }
                        }
                    }
                }
                Ok(Message::Pong(_)) => {
                    // Protocol-level pong received, also update pong time
                    manager_for_recv.update_pong_time(connection_id).await;
                    debug!("Received protocol pong from connection {}", connection_id);
                }
                Ok(Message::Ping(_)) => {
                    debug!("Received protocol ping from connection {}, will auto-respond", connection_id);
                    // Axum automatically handles ping/pong at protocol level
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection {} closed by client", connection_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for connection {}: {}", connection_id, e);
                    break;
                }
                _ => {}
            }
        }

        // Clean up connection
        manager_for_recv.remove_connection(connection_id).await;
    });

    // Wait for any task to complete
    tokio::select! {
        _ = (&mut send_task) => {
            debug!("Send task ended for connection {}", connection_id);
            recv_task.abort();
            ping_task.abort();
        }
        _ = (&mut recv_task) => {
            debug!("Recv task ended for connection {}", connection_id);
            send_task.abort();
            ping_task.abort();
        }
        _ = (&mut ping_task) => {
            debug!("Ping task ended for connection {}", connection_id);
            send_task.abort();
            recv_task.abort();
        }
    }

    info!("WebSocket connection {} fully closed", connection_id);
}

async fn handle_client_message(manager: &UnifiedWebSocketManager, connection_id: Uuid, message: ClientMessage) {
    match message {
        ClientMessage::Subscribe { channels } => match manager.update_subscriptions(connection_id, channels.clone(), true).await {
            Ok(changed) => {
                if !changed.is_empty() {
                    let connections = manager.connections.read().await;
                    if let Some(conn) = connections.get(&connection_id) {
                        let _ = conn.sender.send(ServerMessage::Subscribed { channels: changed });
                    }
                }
            }
            Err(e) => {
                error!("Failed to subscribe: {}", e);
            }
        },

        ClientMessage::Unsubscribe { channels } => match manager.update_subscriptions(connection_id, channels.clone(), false).await {
            Ok(changed) => {
                if !changed.is_empty() {
                    let connections = manager.connections.read().await;
                    if let Some(conn) = connections.get(&connection_id) {
                        let _ = conn.sender.send(ServerMessage::Unsubscribed { channels: changed });
                    }
                }
            }
            Err(e) => {
                error!("Failed to unsubscribe: {}", e);
            }
        },

        ClientMessage::UpdateFilters { filters } => match manager.update_filters(connection_id, filters).await {
            Ok(()) => {
                debug!("Updated filters for connection {}", connection_id);
            }
            Err(e) => {
                error!("Failed to update filters: {}", e);
            }
        },

        ClientMessage::Ping => {
            debug!("Received ping from connection {}, sending pong", connection_id);
            let connections = manager.connections.read().await;
            if let Some(conn) = connections.get(&connection_id) {
                let _ = conn.sender.send(ServerMessage::Pong);
            }
        }

        ClientMessage::Pong => {
            // This is handled in the main recv_task to update last_pong_at
            debug!("Pong message already handled in recv_task for connection {}", connection_id);
        }
    }
}
