use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use qed_core::job::id::{ProvingJobCircuitType, QJobTopic};
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
        .route("/stats/realms", get(global_realm_stats_handler))
        .route("/stats/realms/{realm_id}", get(realm_stats_handler))
        .route(
            "/stats/workers/{worker_public_key}",
            get(worker_stats_handler),
        )
        .route("/rewards/{worker_public_key}", get(worker_rewards_handler))
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
    tracing::info!(
        "User registration request received for public_key: {}, twitter_handle: {}",
        payload.public_key,
        payload.twitter_handle
    );
    tracing::debug!("Register payload: {:#?}", payload);

    // Check if user already exists
    match UserRepository::find_by_public_key(&service.pool, &payload.public_key).await {
        Ok(Some(existing_user)) => {
            tracing::warn!(
                "User registration failed: user already exists with public_key: {}",
                payload.public_key
            );
            tracing::info!("Existing user: {:#?}", existing_user);
            return Ok(Json(RegisterResponse {
                success: false,
                user_id: None,
            }));
        }
        Ok(None) => {
            tracing::warn!("Public key not found, proceeding with registration");
        }
        Err(e) => {
            tracing::error!("Database error while checking existing user: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // TODO: Implement Twitter OAuth verification
    // For now, we just validate that twitter_handle is provided
    if payload.twitter_handle.is_empty() {
        tracing::warn!("Registration failed: empty twitter_handle");
        return Err(StatusCode::BAD_REQUEST);
    }

    // TODO: Implement signature verification
    // The signature should be verified against the public key and a challenge message
    if payload.signature.is_empty() {
        tracing::warn!("Registration failed: empty signature");
        return Err(StatusCode::BAD_REQUEST);
    }

    tracing::info!("Validation passed, proceeding with user creation");

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
            let user_id = user.id.unwrap().to_string();
            tracing::info!(
                "User created successfully: user_id={}, public_key={}, twitter_handle={}",
                user_id,
                payload.public_key,
                payload.twitter_handle
            );

            // Create user registration event
            match UserEventRepository::create(
                &service.pool,
                &user_id,
                &payload.public_key,
                UserEventTxType::RegisterUser,
                None,
                chrono::Utc::now(),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(
                        "User registration event created successfully, user_id={}, public_key={}",
                        user_id,
                        payload.public_key
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to create user registration event: {}", e);
                }
            }

            Ok(Json(RegisterResponse {
                success: true,
                user_id: Some(user_id),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    tracing::info!("User info request for public_key: {}", query.public_key);
    match UserRepository::find_by_public_key(&service.pool, &query.public_key).await {
        Ok(Some(user)) => {
            tracing::info!("User info found: {:#?}", user);
            Ok(Json(user))
        }
        Ok(None) => {
            tracing::warn!("User not found for public_key: {}", query.public_key);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("Database error while fetching user info: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkerEventsQuery {
    pub realm_id: Option<u64>,
    pub status: Option<WorkerEventStatus>,
    pub public_key: Option<String>,
    pub topic: Option<QJobTopic>,
    pub circuit_type: Option<ProvingJobCircuitType>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

async fn worker_events_handler(
    State(service): State<ApiService>,
    Query(query): Query<WorkerEventsQuery>,
) -> Result<Json<Vec<WorkerEvent>>, StatusCode> {
    tracing::info!("Worker events query: {:?}", query);
    let realm_id_i64 = query.realm_id.map(|id| id as i64);

    match WorkerEventRepository::list(
        &service.pool,
        realm_id_i64,
        query.status,
        None, // source filter not provided in query params yet
        query.topic,
        query.circuit_type,
        query.start_time,
        query.end_time,
        0,   // offset
        100, // limit
    )
    .await
    {
        Ok(events) => {
            tracing::info!("Retrieved {} worker events", events.len());
            Ok(Json(events))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve worker events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
    tracing::info!("User events query: {:?}", query);
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
        Err(e) => {
            tracing::error!("Failed to retrieve user events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AggregationQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub bucket: String, // e.g., "1h", "1d", "1w"
}

async fn worker_events_aggregations_handler(
    State(service): State<ApiService>,
    Query(query): Query<AggregationQuery>,
) -> Result<Json<Vec<WorkerEventAggregation>>, StatusCode> {
    tracing::info!("Worker events aggregations query: {:?}", query);
    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "1h" => "worker_events_1h",
        "1d" => "worker_events_1d",
        "1w" => "worker_events_1w",
        "1m" => "worker_events_1m",
        "all_time" => "worker_events_all_time",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match WorkerEventAggregationRepository::get_aggregations(
        &service.pool,
        view_name,
        None, // realm_id filter
        None, // source filter
        query.start_time,
        query.end_time,
        100, // limit
    )
    .await
    {
        Ok(aggregations) => Ok(Json(aggregations)),
        Err(err) => {
            tracing::error!("Failed to retrieve worker events aggregations: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn user_events_aggregations_handler(
    State(service): State<ApiService>,
    Query(query): Query<AggregationQuery>,
) -> Result<Json<Vec<UserEventAggregation>>, StatusCode> {
    tracing::info!("User events aggregations query: {:?}", query);
    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "1h" => "user_events_1h",
        "1d" => "user_events_1d",
        "1w" => "user_events_1w",
        "1m" => "user_events_1m",
        "all_time" => "user_events_all_time",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match UserEventAggregationRepository::get_aggregations(
        &service.pool,
        view_name,
        query.start_time,
        query.end_time,
        100, // limit
    )
    .await
    {
        Ok(aggregations) => Ok(Json(aggregations)),
        Err(e) => {
            tracing::error!("Failed to retrieve user events aggregations: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn stats_handler(
    State(service): State<ApiService>,
) -> Result<Json<HashMap<String, serde_json::Value>>, StatusCode> {
    tracing::info!("Stats request received");
    let mut stats = HashMap::new();

    // Get basic stats using Repository
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::hours(24);
    tracing::info!("Generating stats for period: {} to {}", yesterday, now);

    // Count recent worker events
    let worker_events_count = match WorkerEventRepository::count(
        &service.pool,
        None, // realm_id
        None, // status
        None, // source
        None, // topic
        None, // circuit_type
        Some(yesterday),
        Some(now),
    )
    .await
    {
        Ok(count) => {
            tracing::info!("Worker events count (24h): {}", count);
            count
        }
        Err(e) => {
            tracing::error!("Failed to count worker events: {}", e);
            0
        }
    };

    // Count recent user events
    let user_events = match UserEventRepository::list(
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
    {
        Ok(events) => {
            tracing::info!("User events count (24h): len={}", events.len());
            events
        }
        Err(e) => {
            tracing::error!("Failed to list user events: {}", e);
            Vec::new()
        }
    };

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
    
    // Get block height (maximum checkpoint ID from worker_events)
    let block_height = match TpsRepository::get_max_checkpoint(&service.pool).await {
        Ok(height) => {
            tracing::info!("Block height (max checkpoint): {}", height);
            height
        }
        Err(e) => {
            tracing::error!("Failed to get max checkpoint: {}", e);
            0
        }
    };
    stats.insert(
        "block_height".to_string(),
        serde_json::Value::Number(block_height.into()),
    );

    tracing::info!(
        "Stats generated successfully: worker_events_24h={}, user_events_24h={}, block_height={}",
        worker_events_count,
        user_events.len(),
        block_height
    );
    tracing::debug!("Stats response: {:?}", stats);

    Ok(Json(stats))
}

async fn realm_stats_handler(
    State(service): State<ApiService>,
    Path(realm_id): Path<i64>,
) -> Result<Json<RealmStats>, StatusCode> {
    tracing::info!("Realm stats request for realm_id: {}", realm_id);
    match RealmStatsRepository::get_realm_stats(&service.pool, realm_id).await {
        Ok(stats) => {
            tracing::info!("Retrieved realm stats: {:#?}", stats);
            Ok(Json(stats))
        }
        Err(e) => {
            tracing::error!(
                "Failed to retrieve realm stats for realm_id {}: {}",
                realm_id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn global_realm_stats_handler(
    State(service): State<ApiService>,
) -> Result<Json<GlobalRealmStats>, StatusCode> {
    tracing::info!("Global realm stats request received");
    match RealmStatsRepository::get_global_realm_stats(&service.pool).await {
        Ok(stats) => {
            tracing::info!("Retrieved global realm stats: {:#?}", stats);
            Ok(Json(stats))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve global realm stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn worker_stats_handler(
    State(service): State<ApiService>,
    Path(worker_public_key): Path<String>,
) -> Result<Json<WorkerStats>, StatusCode> {
    tracing::info!("Worker stats request for worker: {}", worker_public_key);

    match WorkerStatsRepository::get_worker_stats(&service.pool, &worker_public_key).await {
        Ok(stats) => {
            tracing::info!("Retrieved worker stats: {:#?}", stats);
            Ok(Json(stats))
        }
        Err(e) => {
            tracing::error!(
                "Failed to retrieve worker stats for worker {}: {}",
                worker_public_key,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkerRewardsQuery {
    pub checkpoint_id: i64,
}

async fn worker_rewards_handler(
    State(service): State<ApiService>,
    Path(worker_public_key): Path<String>,
    Query(query): Query<WorkerRewardsQuery>,
) -> Result<Json<WorkerRewards>, StatusCode> {
    tracing::info!(
        "Worker rewards request for worker: {}, checkpoint_id: {}",
        worker_public_key,
        query.checkpoint_id
    );

    match WorkerRewardsRepository::get_worker_rewards(
        &service.pool,
        &worker_public_key,
        query.checkpoint_id,
    )
    .await
    {
        Ok(rewards) => {
            tracing::info!("Retrieved worker rewards: {:#?}", rewards);
            Ok(Json(rewards))
        }
        Err(e) => {
            tracing::error!(
                "Failed to retrieve worker rewards for worker {} with checkpoint_id {}: {}",
                worker_public_key,
                query.checkpoint_id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
