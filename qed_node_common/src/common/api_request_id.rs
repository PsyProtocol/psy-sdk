use chrono::Utc;
use qed_data::api::request_id::{QAPIWriteRequestBlobType, QAPIWriteRequestType, QEDAPIWriteRequestId, WithRequestId};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};



#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash, PartialOrd, Ord,
)]
pub struct QEDAPIRequestIdGenerator {
    pub realm_id: u32,
    pub node_id: u32,
}

impl QEDAPIRequestIdGenerator {
    pub fn new(realm_id: u32, node_id: u32) -> Self {
        Self {
            realm_id,
            node_id,
        }
    }
    pub fn new_request_id(&self, request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType) -> QEDAPIWriteRequestId {
        QEDAPIWriteRequestId::new_rand(request_type, data_type, self.realm_id, self.node_id)
    }
    pub fn wrap_with_req_id<T>(&self, request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType, payload: T) -> WithRequestId<T> {
        WithRequestId { id: self.new_request_id(request_type, data_type), payload, }
    }
    
}

pub trait QEDAPIWriteRequestIdGen {
    fn new_rand(request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType, realm_id: u32, node_id: u32) -> QEDAPIWriteRequestId;
}

impl QEDAPIWriteRequestIdGen for QEDAPIWriteRequestId {
    fn new_rand(request_type: QAPIWriteRequestType, data_type: QAPIWriteRequestBlobType, realm_id: u32, node_id: u32) -> Self {
        let random = thread_rng().gen::<u64>();
        let time = Utc::now().timestamp_millis() as u64;

        Self {
            request_type,
            data_type,
            realm_id,
            node_id,
            time,
            random,
        }
    }
    
}