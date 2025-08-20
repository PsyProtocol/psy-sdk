use std::sync::Arc;

use crate::db::DatabaseConnections;

#[derive(Clone)]
pub struct ApiService {
    pub db: Option<Arc<DatabaseConnections>>,
    // TODO: Add repository instances
    // TODO: Add HTTP clients for coordinator/realm communication
}

impl ApiService {
    pub fn new(db: DatabaseConnections) -> Self {
        Self {
            db: Some(Arc::new(db)),
        }
    }

    pub fn new_mock() -> Self {
        Self {
            db: None,
        }
    }

    // TODO: Implement service methods for business logic
}