use sqlx::PgPool;

#[derive(Clone)]
pub struct ApiService {
    pub db: PgPool,
    // TODO: Add repository instances
    // TODO: Add HTTP clients for coordinator/realm communication
}

impl ApiService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub fn new_mock() -> Self {
        todo!("temp mocking")
    }

    // TODO: Implement service methods for business logic
}
