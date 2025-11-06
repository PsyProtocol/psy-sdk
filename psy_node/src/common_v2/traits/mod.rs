pub mod realm;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PRRealmToCoordinatorMessageType {
    Ping = 0,
    Pong = 1,
    SubmitCompletedRealmProof = 2,
}
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PRCoordinatorToRealmMessageType {
    Ping = 0,
    Pong = 1,
    BlockCompleted = 2,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct PRRealmToCoordinatorMessage {
    pub message_type: PRRealmToCoordinatorMessageType,
    pub sender_realm_id: u64,
    pub sender_realm_manager_id: u64,
    pub sender_realm_checkpoint_id: u64,
    pub sent_time_ms: u64,
    pub payload: Vec<u8>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct PRCoordinatorToRealmMessage {
    pub message_type: PRCoordinatorToRealmMessageType,
    pub sender_coordinator_manager_id: u64,
    pub sender_coordinator_checkpoint_id: u64,
    pub sent_time_ms: u64,
    pub payload: Vec<u8>,
}

pub trait PRCoordinatorClient {
    async fn send_message_to_coordinator_async(&self, message: PRRealmToCoordinatorMessage) -> anyhow::Result<()>;
    async fn wait_for_messsage_from_coordinator_async(&self) -> anyhow::Result<PRCoordinatorToRealmMessage>;
    fn is_connected_to_coordinator(&self) -> bool;
}
pub trait PRRealmClient {
    async fn send_message_to_realm_async(&self, message: PRCoordinatorToRealmMessage) -> anyhow::Result<()>;
    async fn wait_for_message_from_realm_async(&self) -> anyhow::Result<PRRealmToCoordinatorMessage>;
    fn is_connected_to_realm(&self) -> bool;
}

pub trait PRRealmToCoordinatorHandler: PRCoordinatorClient + Send + Sync {
    fn get_realm_id(&self) -> u64;
    fn get_sender_realm_manager_id(&self) -> u64;
    fn get_realm_checkpoint_id(&self) -> u64;
    fn get_realm_time_ms(&self) -> u64;
}
