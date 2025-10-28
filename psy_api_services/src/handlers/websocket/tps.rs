use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::time::{interval, Duration};

use crate::{repositories::TpsRepository, services::ApiService};

use super::{EventType, WebSocketEvent};

pub async fn websocket_tps_handler(
    ws: WebSocketUpgrade,
    State(service): State<ApiService>,
) -> Response {
    ws.on_upgrade(move |socket| handle_tps_socket(socket, service))
}

async fn handle_tps_socket(socket: WebSocket, service: ApiService) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let connection_id = uuid::Uuid::new_v4().to_string();

    tracing::info!(
        "New TPS WebSocket connection established: {}",
        connection_id
    );

    // Create interval for 12-second broadcasts
    let mut broadcast_interval = interval(Duration::from_secs(12));

    tokio::select! {
        // Handle periodic TPS broadcasts
        _ = async {
            loop {
                broadcast_interval.tick().await;

                match TpsRepository::calculate_current_tps(&service.pool).await {
                    Ok(tps_data) => {
                        let event = WebSocketEvent {
                            event_type: EventType::TpsUpdate,
                            data: serde_json::to_value(&tps_data).unwrap_or_default(),
                            timestamp: tps_data.timestamp,
                        };

                        let message = Message::Text(
                            serde_json::to_string(&event).unwrap_or_default().into()
                        );

                        if let Err(e) = ws_sender.send(message).await {
                            tracing::warn!("Failed to send TPS update to connection {}: {}", connection_id, e);
                            break;
                        }

                        tracing::info!("Sent TPS update to connection {}: TPS = {:.2}", connection_id, tps_data.tps);
                    }
                    Err(e) => {
                        tracing::error!("Failed to calculate TPS for connection {}: {}", connection_id, e);
                        // Continue even if one calculation fails
                    }
                }
            }
        } => {}

        // Handle incoming messages (mostly to detect disconnection)
        _ = async {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Close(_)) => {
                        tracing::info!("TPS WebSocket connection closed: {}", connection_id);
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        tracing::trace!("Received ping from TPS connection: {}", connection_id);
                    }
                    Ok(Message::Pong(_)) => {
                        tracing::trace!("Received pong from TPS connection: {}", connection_id);
                    }
                    Err(e) => {
                        tracing::error!("TPS WebSocket error for connection {}: {}", connection_id, e);
                        break;
                    }
                    _ => {
                        tracing::trace!("Received message from TPS connection: {}", connection_id);
                    }
                }
            }
        } => {}
    }

    tracing::info!("TPS WebSocket connection ended: {}", connection_id);
}
