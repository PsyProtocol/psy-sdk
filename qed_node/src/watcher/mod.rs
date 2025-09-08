use clap::Args;

pub mod api_client;
pub mod watcher;
pub mod events;
pub mod config;
pub mod block_height;
pub mod watcher_client;
pub mod watcher_service;
pub mod schedule_tasks;


pub use api_client::*;
pub use config::*;
pub use watcher_client::*;
pub use watcher_service::*;
pub use block_height::*;
pub use schedule_tasks::*;

#[derive(Debug, Clone, Copy)]
pub enum QedNodeType {
    Coordinator,
    Realm,
    Worker,
}