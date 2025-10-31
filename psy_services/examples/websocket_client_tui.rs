// Complete fixed main function - removes ui_task spawn

use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time::interval};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message as WsMessage, Utf8Bytes},
};

// ============================================================================
// Message Types (must match server)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    UpdateFilters { filters: Filters },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Event {
        channel: String,
        data: serde_json::Value,
        timestamp: String,
    },
    Subscribed {
        channels: Vec<String>,
    },
    Unsubscribed {
        channels: Vec<String>,
    },
    Error {
        code: String,
        message: String,
    },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Filters {
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_realm_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_type: Option<String>,
}

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "websocket_client_tui")]
#[command(about = "TUI WebSocket client for testing unified WebSocket endpoint")]
struct Args {
    #[arg(long, default_value = "ws://localhost:3000/ws")]
    url: String,

    #[arg(long, default_value = "worker_events,user_events,tps")]
    channels: String,

    #[arg(long)]
    worker_realm_id: Option<i64>,

    #[arg(long)]
    worker_public_key: Option<String>,

    #[arg(long, default_value = "30")]
    ping_interval: u64,
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Debug, Clone)]
enum LogMessage {
    Sent(String),
    Received(String),
    Error(String),
}

#[derive(Debug, Clone, Default)]
struct Statistics {
    worker_events: u64,
    user_events: u64,
    tps_updates: u64,
    pings_sent: u64,
    pongs_received: u64,
    server_pings_received: u64,
    pongs_sent: u64,
    errors: u64,
}

struct App {
    logs: Vec<LogMessage>,
    stats: Statistics,
    scroll_offset: usize,
    max_logs: usize,
    connection_status: String,
}

impl App {
    fn new() -> Self {
        Self {
            logs: Vec::new(),
            stats: Statistics::default(),
            scroll_offset: 0,
            max_logs: 1000,
            connection_status: "Connecting...".to_string(),
        }
    }

    fn add_log(&mut self, log: LogMessage) {
        self.logs.push(log);
        if self.logs.len() > self.max_logs {
            self.logs.remove(0);
        }
        // Auto-scroll to bottom
        self.scroll_offset = self.logs.len().saturating_sub(1);
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        if self.scroll_offset < self.logs.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }
}

// ============================================================================
// UI Rendering
// ============================================================================

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Messages
            Constraint::Length(12), // Stats
        ])
        .split(f.size());

    // Header
    render_header(f, chunks[0], app);

    // Messages
    render_messages(f, chunks[1], app);

    // Statistics
    render_stats(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("QED WebSocket TUI Client", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(
                &app.connection_status,
                if app.connection_status.contains("Connected") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            ),
        ]),
        Line::from(vec![Span::styled("Press 'q' or 'Esc' to quit", Style::default().fg(Color::DarkGray))]),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Left);

    f.render_widget(header, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    // Calculate how many messages can fit in the area
    let available_height = area.height.saturating_sub(2) as usize; // Subtract borders

    // Get the last N messages that fit in the view
    let start_idx = app.logs.len().saturating_sub(available_height);
    let visible_logs = &app.logs[start_idx..];

    let messages: Vec<ListItem> = visible_logs
        .iter()
        .map(|log| {
            let (prefix, text, style) = match log {
                LogMessage::Sent(msg) => ("→", msg.as_str(), Style::default().fg(Color::Yellow)),
                LogMessage::Received(msg) => {
                    // Color-code by event type
                    let color = if msg.contains("WORKER EVENT") {
                        Color::Green
                    } else if msg.contains("USER EVENT") {
                        Color::Yellow
                    } else if msg.contains("TPS UPDATE") {
                        Color::LightMagenta
                    } else {
                        Color::Cyan
                    };
                    ("←", msg.as_str(), Style::default().fg(color))
                }
                LogMessage::Error(msg) => ("✗", msg.as_str(), Style::default().fg(Color::LightRed)),
            };

            let content = format!("{} {}", prefix, text);
            ListItem::new(Text::from(content)).style(style)
        })
        .collect();

    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title(format!(" Messages ({}) ", app.logs.len())))
        .start_corner(ratatui::layout::Corner::TopLeft);

    f.render_widget(messages_widget, area);
}

fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let stats = &app.stats;

    let text = vec![
        Line::from(vec![
            Span::styled("Events: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Worker={} ", stats.worker_events), Style::default().fg(Color::Green)),
            Span::styled(format!("User={} ", stats.user_events), Style::default().fg(Color::Blue)),
            Span::styled(format!("TPS={}", stats.tps_updates), Style::default().fg(Color::Magenta)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Client→Server: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Pings={} ", stats.pings_sent), Style::default().fg(Color::Yellow)),
            Span::styled(format!("Pongs={}", stats.pongs_received), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Server→Client: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Pings={} ", stats.server_pings_received), Style::default().fg(Color::Cyan)),
            Span::styled(format!("Pongs={}", stats.pongs_sent), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Errors: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", stats.errors), Style::default().fg(Color::Red)),
        ]),
    ];

    let stats_widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Statistics "))
        .wrap(Wrap { trim: false });

    f.render_widget(stats_widget, area);
}

// ============================================================================
// Main Function
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = Arc::new(Mutex::new(App::new()));

    // Build URL
    let mut url = format!("{}?channels={}", args.url, args.channels);
    if let Some(realm_id) = args.worker_realm_id {
        url.push_str(&format!("&worker_realm_id={}", realm_id));
    }
    if let Some(ref public_key) = args.worker_public_key {
        url.push_str(&format!("&worker_public_key={}", public_key));
    }

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&url).await?;

    {
        let mut app = app.lock().await;
        app.connection_status = "Connected".to_string();
        app.add_log(LogMessage::Received(format!("Connected to {}", args.url)));
    }

    let (write, mut read) = ws_stream.split();

    // Wrap write in Arc<Mutex> so it can be shared between tasks
    let write = Arc::new(Mutex::new(write));

    // Clone app and write for tasks
    let app_ws = Arc::clone(&app);
    let app_ping = Arc::clone(&app);
    let write_ws = Arc::clone(&write);
    let write_ping = Arc::clone(&write);

    // WebSocket receive task
    let ws_task = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    let mut app = app_ws.lock().await;

                    // Check for server ping
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        if matches!(server_msg, ServerMessage::Ping) {
                            app.stats.server_pings_received += 1;
                            app.add_log(LogMessage::Received("PING from server".to_string()));
                            drop(app);

                            // Send pong
                            let pong_msg = ClientMessage::Pong;
                            let json = serde_json::to_string(&pong_msg).unwrap();
                            let mut write_guard = write_ws.lock().await;
                            let _ = write_guard.send(WsMessage::Text(Utf8Bytes::from(json))).await;
                            drop(write_guard);

                            let mut app = app_ws.lock().await;
                            app.stats.pongs_sent += 1;
                            app.add_log(LogMessage::Sent("PONG response".to_string()));
                            continue;
                        }

                        // Handle other messages
                        handle_server_message(server_msg, &mut app);
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    let mut app = app_ws.lock().await;
                    app.connection_status = "Disconnected".to_string();
                    app.add_log(LogMessage::Error("Connection closed".to_string()));
                    break;
                }
                Err(e) => {
                    let mut app = app_ws.lock().await;
                    app.connection_status = "Error".to_string();
                    app.add_log(LogMessage::Error(format!("WebSocket error: {}", e)));
                    break;
                }
                _ => {}
            }
        }
    });

    // Ping task
    let ping_task = tokio::spawn(async move {
        let mut ping_timer = interval(Duration::from_secs(args.ping_interval));
        ping_timer.tick().await;

        loop {
            ping_timer.tick().await;

            let ping_msg = ClientMessage::Ping;
            let json = serde_json::to_string(&ping_msg).unwrap();

            let mut write_guard = write_ping.lock().await;
            if write_guard.send(WsMessage::Text(Utf8Bytes::from(json))).await.is_err() {
                break;
            }
            drop(write_guard);

            let mut app = app_ping.lock().await;
            app.stats.pings_sent += 1;
            app.add_log(LogMessage::Sent("PING".to_string()));
        }
    });

    // UI event loop (NO tokio::spawn - runs in main task)
    loop {
        // Draw UI
        {
            let app_lock = app.lock().await;
            if terminal.draw(|f| ui(f, &app_lock)).is_err() {
                break;
            }
        }

        // Handle input
        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Up => {
                        let mut app_lock = app.lock().await;
                        app_lock.scroll_up();
                    }
                    KeyCode::Down => {
                        let mut app_lock = app.lock().await;
                        app_lock.scroll_down();
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Cleanup tasks
    ws_task.abort();
    ping_task.abort();

    // Restore terminal (we still own it since UI loop was in main)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

// ============================================================================
// Message Handler
// ============================================================================

fn handle_server_message(msg: ServerMessage, app: &mut App) {
    match msg {
        ServerMessage::Event { channel, data, .. } => match channel.as_str() {
            "worker_events" => {
                app.stats.worker_events += 1;
                let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                app.add_log(LogMessage::Received(format!("WORKER EVENT: status={}", status)));
            }
            "user_events" => {
                app.stats.user_events += 1;
                let user_id = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                app.add_log(LogMessage::Received(format!("USER EVENT: user_id={}", user_id)));
            }
            "tps" => {
                app.stats.tps_updates += 1;
                let tps = data.get("tps").and_then(|v| v.as_f64()).unwrap_or(0.0);
                app.add_log(LogMessage::Received(format!("TPS UPDATE: {:.2}", tps)));
            }
            _ => {
                app.add_log(LogMessage::Received(format!("EVENT: channel={}", channel)));
            }
        },
        ServerMessage::Subscribed { channels } => {
            app.add_log(LogMessage::Received(format!("Subscribed to: {:?}", channels)));
        }
        ServerMessage::Unsubscribed { channels } => {
            app.add_log(LogMessage::Received(format!("Unsubscribed from: {:?}", channels)));
        }
        ServerMessage::Error { code, message } => {
            app.stats.errors += 1;
            app.add_log(LogMessage::Error(format!("[{}] {}", code, message)));
        }
        ServerMessage::Ping => {
            // Handled in main loop
        }
        ServerMessage::Pong => {
            app.stats.pongs_received += 1;
            app.add_log(LogMessage::Received("PONG".to_string()));
        }
    }
}
