use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{models::*, repositories::*, services::ApiService};

pub fn create_router(api_service: ApiService) -> Router {
    Router::new()
        .route("/register", post(register_handler))
        .route("/user_info", get(user_info_handler))
        .route("/worker_events", get(worker_events_handler))
        .route("/user_events", get(user_events_handler))
        .route(
            "/worker_events_aggregations",
            get(worker_events_aggregations_handler),
        )
        .route(
            "/user_events_aggregations",
            get(user_events_aggregations_handler),
        )
        .route("/stats", get(stats_handler))
        .with_state(api_service)
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub public_key: String,
    pub twitter_handle: String,
    pub label: String,
    pub signature: String, // TODO: Implement signature verification
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub user_id: Option<String>,
}

async fn register_handler(
    State(service): State<ApiService>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    // Check if user already exists
    if let Ok(Some(_existing_user)) =
        UserRepository::find_by_public_key(&service.pool, &payload.public_key).await
    {
        return Ok(Json(RegisterResponse {
            success: false,
            user_id: None,
        }));
    }

    // TODO: Implement Twitter OAuth verification
    // For now, we just validate that twitter_handle is provided
    if payload.twitter_handle.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // TODO: Implement signature verification
    // The signature should be verified against the public key and a challenge message
    if payload.signature.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create new user
    match UserRepository::create(
        &service.pool,
        &payload.public_key,
        Some(&payload.twitter_handle),
        Some(&payload.label),
    )
    .await
    {
        Ok(user) => {
            // Create user registration event
            let _ = UserEventRepository::create(
                &service.pool,
                &user.id.unwrap().to_string(),
                &payload.public_key,
                UserEventTxType::RegisterUser,
                None,
                chrono::Utc::now(),
            )
            .await;

            Ok(Json(RegisterResponse {
                success: true,
                user_id: user.id.map(|id| id.to_string()),
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Debug, Deserialize)]
pub struct UserInfoQuery {
    pub public_key: String,
}

async fn user_info_handler(
    State(service): State<ApiService>,
    Query(query): Query<UserInfoQuery>,
) -> Result<Json<UserInfo>, StatusCode> {
    match UserRepository::find_by_public_key(&service.pool, &query.public_key).await {
        Ok(Some(user)) => Ok(Json(user)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkerEventsQuery {
    pub realm_id: Option<u64>,
    pub status: Option<WorkerEventStatus>,
    pub public_key: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

async fn worker_events_handler(
    State(service): State<ApiService>,
    Query(query): Query<WorkerEventsQuery>,
) -> Result<Json<Vec<WorkerEvent>>, StatusCode> {
    let realm_id_i64 = query.realm_id.map(|id| id as i64);

    match WorkerEventRepository::list(
        &service.pool,
        realm_id_i64,
        query.status,
        None, // source filter not provided in query params yet
        query.start_time,
        query.end_time,
        0,   // offset
        100, // limit
    )
    .await
    {
        Ok(events) => Ok(Json(events)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Debug, Deserialize)]
pub struct UserEventsQuery {
    pub user_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub tx_type: Option<UserEventTxType>,
}

async fn user_events_handler(
    State(service): State<ApiService>,
    Query(query): Query<UserEventsQuery>,
) -> Result<Json<Vec<UserEvent>>, StatusCode> {
    match UserEventRepository::list(
        &service.pool,
        query.user_id.as_deref(),
        None, // public_key filter not in query
        query.tx_type,
        query.start_time,
        query.end_time,
        0,   // offset
        100, // limit
    )
    .await
    {
        Ok(events) => Ok(Json(events)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Debug, Deserialize)]
pub struct AggregationQuery {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub bucket: String, // e.g., "1h", "1d", "1w"
}

async fn worker_events_aggregations_handler(
    State(service): State<ApiService>,
    Query(query): Query<AggregationQuery>,
) -> Result<Json<Vec<WorkerEventAggregation>>, StatusCode> {
    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "1h" => "worker_events_1h",
        "1d" => "worker_events_1d",
        "1w" => "worker_events_1w",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match WorkerEventAggregationRepository::get_aggregations(
        &service.pool,
        view_name,
        None, // realm_id filter
        None, // source filter
        Some(query.start_time),
        Some(query.end_time),
        100, // limit
    )
    .await
    {
        Ok(aggregations) => Ok(Json(aggregations)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn user_events_aggregations_handler(
    State(service): State<ApiService>,
    Query(query): Query<AggregationQuery>,
) -> Result<Json<Vec<UserEventAggregation>>, StatusCode> {
    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "1h" => "user_events_1h",
        "1d" => "user_events_1d",
        "1w" => "user_events_1w",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match UserEventAggregationRepository::get_aggregations(
        &service.pool,
        view_name,
        Some(query.start_time),
        Some(query.end_time),
        100, // limit
    )
    .await
    {
        Ok(aggregations) => Ok(Json(aggregations)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn stats_handler(
    State(service): State<ApiService>,
) -> Result<Json<HashMap<String, serde_json::Value>>, StatusCode> {
    let mut stats = HashMap::new();

    // Get basic stats using Repository
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::hours(24);

    // Count recent worker events
    let worker_events_count = WorkerEventRepository::count(
        &service.pool,
        None, // realm_id
        None, // status
        None, // source
        Some(yesterday),
        Some(now),
    )
    .await
    .unwrap_or(0);

    // Count recent user events
    let user_events = UserEventRepository::list(
        &service.pool,
        None, // user_id
        None, // public_key
        None, // tx_type
        Some(yesterday),
        Some(now),
        0,    // offset
        1000, // large limit to get count
    )
    .await
    .unwrap_or_default();

    stats.insert(
        "status".to_string(),
        serde_json::Value::String("ok".to_string()),
    );
    stats.insert(
        "worker_events_24h".to_string(),
        serde_json::Value::Number(worker_events_count.into()),
    );
    stats.insert(
        "user_events_24h".to_string(),
        serde_json::Value::Number((user_events.len() as i64).into()),
    );
    stats.insert(
        "timestamp".to_string(),
        serde_json::Value::String(now.to_rfc3339()),
    );

    Ok(Json(stats))
}
