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
use crate::repositories::checkpoint_state::{CheckpointRewardDistributionRepository, CheckpointStatsRepository, WorkerJobEventRepository};
use crate::repositories::rewards::WorkerRewardsRepository;
use crate::services::{CheckpointRewardService, JobStatusService, TimePeriod};

/// Parse order parameter from query string
/// Returns true for ASC, false for DESC (default)
fn parse_order_param(order: Option<&str>) -> bool {
    match order {
        Some(s) if s.eq_ignore_ascii_case("asc") => true,
        Some(s) if s.eq_ignore_ascii_case("desc") => false,
        None => false, // default to DESC
        Some(invalid) => {
            tracing::warn!("Invalid order parameter: '{}', using default DESC", invalid);
            false
        }
    }
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn create_router(api_service: ApiService) -> Router {
    Router::new()
        // Health & User Management
        .route("/health", get(health_handler))
        .route("/register", post(register_handler))
        .route("/user_info", get(user_info_handler))

        // Events
        .route("/worker_events", get(worker_events_handler))
        .route("/user_events", get(user_events_handler))
        .route("/worker_events_aggregations", get(worker_events_aggregations_handler))
        .route("/user_events_aggregations", get(user_events_aggregations_handler))

        // Stats - General
        .route("/stats", get(stats_handler))
        .route("/stats/realms", get(global_realm_stats_handler))
        .route("/stats/realms/{realm_id}", get(realm_stats_handler))
        .route("/stats/workers/{worker_public_key}", get(worker_stats_handler))

        // Stats - Job Status
        .route("/stats/jobs", get(job_status_summary_handler))
        .route("/stats/jobs/realm/{realm_id}", get(realm_job_status_handler))
        .route("/stats/jobs/all-realms", get(all_realms_job_status_handler))
        .route("/stats/jobs/counts", get(job_counts_handler))

        // Legacy Rewards (old system - consider deprecating)
        .route("/rewards/{worker_public_key}", get(worker_rewards_handler))
        .route("/rewards_aggregations/{worker_public_key}", get(worker_rewards_aggregations_handler))

        // Leaderboard
        .route("/leaderboard/workers", get(worker_leaderboard_handler))

        // Checkpoint Stats - QUERIES ONLY
        .route("/checkpoint/stats", get(get_checkpoint_stats_by_range_handler))
        .route("/checkpoint/stats/{checkpoint_id}", get(get_checkpoint_stats_handler))

        // Worker Job Events
        .route("/checkpoint/job-events/{checkpoint_id}", get(get_checkpoint_job_events_handler))

        // Reward Calculation & Distribution
        .route("/checkpoint/distributions/{checkpoint_id}", get(get_checkpoint_distributions_handler))
        .route("/checkpoint/summary/{checkpoint_id}", get(get_checkpoint_summary_handler))

        // Worker Rewards
        .route("/checkpoint/rewards/{worker_public_key}", get(get_worker_rewards_handler))
        .route("/checkpoint/rewards/{worker_public_key}/stats", get(get_worker_reward_stats_handler))

        // Admin Operations
        .route("/checkpoint/calculate-rewards/{checkpoint_id}", post(calculate_rewards_handler))
        .route("/admin/checkpoint-processing-status", get(checkpoint_processing_status_handler))

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
    pub from_checkpoint_id: Option<i64>,
    pub to_checkpoint_id: Option<i64>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub order: Option<String>, // "asc" or "desc", default "desc"
    pub category: Option<JobFilterCategory>,
}

async fn worker_events_handler(
    State(service): State<ApiService>,
    Query(query): Query<WorkerEventsQuery>,
) -> Result<Json<Vec<WorkerEvent>>, StatusCode> {
    tracing::info!("Worker events query: {:?}", query);
    let realm_id_i64 = query.realm_id.map(|id| id as i64);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(300).clamp(1, 1000);
    let order_asc = parse_order_param(query.order.as_deref());

    let filter_category = query.category.unwrap_or_default();
    tracing::info!("Using filter category: {:?}", filter_category);

    match WorkerEventRepository::list(
        &service.pool,
        realm_id_i64,
        query.public_key,
        query.status,
        None, // source filter not provided in query params yet
        query.topic,
        query.circuit_type,
        query.from_checkpoint_id,
        query.to_checkpoint_id,
        filter_category,
        offset,
        limit,
        order_asc,
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
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub order: Option<String>, // "asc" or "desc", default "desc"
}

async fn user_events_handler(
    State(service): State<ApiService>,
    Query(query): Query<UserEventsQuery>,
) -> Result<Json<Vec<UserEvent>>, StatusCode> {
    tracing::info!("User events query: {:?}", query);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(300).clamp(1, 1000);
    let order_asc = parse_order_param(query.order.as_deref());

    match UserEventRepository::list(
        &service.pool,
        query.user_id.as_deref(),
        None, // public_key filter not in query
        query.tx_type,
        query.start_time,
        query.end_time,
        offset,
        limit,
        order_asc,
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
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub order: Option<String>, // "asc" or "desc", default "desc"
}

async fn worker_events_aggregations_handler(
    State(service): State<ApiService>,
    Query(query): Query<AggregationQuery>,
) -> Result<Json<Vec<WorkerEventAggregation>>, StatusCode> {
    tracing::info!("Worker events aggregations query: {:?}", query);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(300).clamp(1, 1000);
    let order_asc = parse_order_param(query.order.as_deref());

    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "2min" => "worker_events_2min",
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
        offset,
        limit,
        order_asc,
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
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(300).clamp(1, 1000);
    let order_asc = parse_order_param(query.order.as_deref());

    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "2min" => "user_events_2min",
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
        offset,
        limit,
        order_asc,
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
    let user_events_count = match UserEventRepository::count(
        &service.pool,
        None, // user_id
        None, // public_key
        None, // tx_type
        Some(yesterday),
        Some(now),
    )
    .await
    {
        Ok(count) => {
            tracing::info!("User events count (24h): {}", count);
            count
        }
        Err(e) => {
            tracing::error!("Failed to count user events: {}", e);
            0
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
        serde_json::Value::Number(user_events_count.into()),
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
        user_events_count,
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
    let checkpoint_id = query.checkpoint_id;

    tracing::info!(
        "Worker rewards request for worker: {}, checkpoint_id: {}",
        worker_public_key,
        checkpoint_id
    );

    match WorkerRewardsRepository::get_worker_rewards(
        &service.pool,
        &worker_public_key,
        checkpoint_id,
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
                checkpoint_id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkerLeaderboardQuery {
    pub limit: Option<i64>, // Number of top workers to return (default 100, max 100)
}

async fn worker_leaderboard_handler(
    State(service): State<ApiService>,
    Query(query): Query<WorkerLeaderboardQuery>,
) -> Result<Json<Vec<WorkerLeaderboardEntry>>, StatusCode> {
    // Validate and set limit (default 100, max 100)
    let limit = query.limit.unwrap_or(100).min(100).max(1);

    tracing::info!("Worker leaderboard request received, limit: {}", limit);

    match WorkerLeaderboardRepository::get_leaderboard_24h(&service.pool, limit).await {
        Ok(leaderboard) => {
            tracing::info!("Retrieved {} leaderboard entries", leaderboard.len());
            tracing::debug!("Leaderboard entries: {:#?}", leaderboard);
            Ok(Json(leaderboard))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve worker leaderboard: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkerRewardsAggregationQuery {
    pub bucket: String, // e.g., "1d", "1w", "1m"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>, // Number of buckets to return (default 100)
}

async fn worker_rewards_aggregations_handler(
    State(service): State<ApiService>,
    Path(worker_public_key): Path<String>,
    Query(query): Query<WorkerRewardsAggregationQuery>,
) -> Result<Json<Vec<WorkerRewardsAggregation>>, StatusCode> {
    tracing::info!(
        "Worker rewards aggregations request for worker: {}, bucket: {}",
        worker_public_key,
        query.bucket
    );

    // Determine view name based on bucket interval
    let view_name = match query.bucket.as_str() {
        "1m" => "worker_rewards_1m",
        "1d" => "worker_rewards_1d",
        "1w" => "worker_rewards_1w",
        "all_time" => "worker_rewards_all_time",
        _ => {
            tracing::warn!("Invalid bucket parameter: {}", query.bucket);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let limit = query.limit.unwrap_or(100).min(1000).max(1);

    match WorkerRewardsAggregationRepository::get_aggregations(
        &service.pool,
        view_name,
        &worker_public_key,
        query.start_time,
        query.end_time,
        limit,
    )
    .await
    {
        Ok(aggregations) => {
            tracing::info!(
                "Retrieved {} worker rewards aggregation entries for worker: {}",
                aggregations.len(),
                worker_public_key
            );
            Ok(Json(aggregations))
        }
        Err(e) => {
            tracing::error!(
                "Failed to retrieve worker rewards aggregations for worker {}: {}",
                worker_public_key,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


#[derive(Debug, Deserialize)]
pub struct RefreshAggregatesRequest {
    pub aggregate_type: Option<String>, // Optional: specific aggregate to refresh, or all if not specified
}


// Query parameters for job status endpoint
#[derive(Debug, Deserialize)]
pub struct JobStatusQuery {
    /// Optional: filter by time window (in hours)
    pub hours: Option<u32>,
    /// Optional: filter by specific realm
    pub realm_id: Option<i64>,
}

// Response structure for job status
#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub summary: Vec<JobStatusSummary>,
    pub total_jobs: i64,
    pub query_time: DateTime<Utc>,
    pub materialized_view_healthy: bool,
}

/// GET /stats/jobs - Get job status summary
async fn job_status_summary_handler(
    State(service): State<ApiService>,
    Query(query): Query<JobStatusQuery>,
) -> Result<Json<JobStatusResponse>, StatusCode> {
    tracing::info!("Job status summary request: {:?}", query);

    // Check materialized view health
    let view_healthy = JobStatusRepository::check_materialized_view_health(&service.pool)
        .await
        .unwrap_or(false);

    if !view_healthy {
        tracing::warn!("❗ Materialized view 'latest_job_status' is not healthy or empty");
    }

    let summary = if let Some(hours) = query.hours {
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        JobStatusRepository::get_job_status_summary_with_time_window(&service.pool, since).await
    } else if let Some(realm_id) = query.realm_id {
        JobStatusRepository::get_job_status_summary_by_realm(&service.pool, Some(realm_id)).await
    } else {
        JobStatusRepository::get_job_status_summary(&service.pool).await
    };

    match summary {
        Ok(summary) => {
            let total_jobs: i64 = summary.iter().map(|s| s.job_count).sum();

            tracing::info!(
                "Retrieved job status summary: {} statuses, {} total jobs",
                summary.len(),
                total_jobs
            );

            Ok(Json(JobStatusResponse {
                summary,
                total_jobs,
                query_time: Utc::now(),
                materialized_view_healthy: view_healthy,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve job status summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /stats/jobs/realm/:realm_id - Get job status for a specific realm
async fn realm_job_status_handler(
    State(service): State<ApiService>,
    Path(realm_id): Path<i64>,
) -> Result<Json<Vec<JobStatusSummary>>, StatusCode> {
    tracing::info!("Realm job status request for realm_id: {}", realm_id);

    match JobStatusRepository::get_job_status_summary_by_realm(&service.pool, Some(realm_id)).await {
        Ok(summary) => {
            tracing::info!(
                "Retrieved job status for realm {}: {} statuses",
                realm_id,
                summary.len()
            );
            Ok(Json(summary))
        }
        Err(e) => {
            tracing::error!(
                "Failed to retrieve job status for realm {}: {}",
                realm_id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /stats/jobs/all-realms - Get job status grouped by all realms
async fn all_realms_job_status_handler(
    State(service): State<ApiService>,
) -> Result<Json<Vec<RealmJobStatusSummary>>, StatusCode> {
    tracing::info!("All realms job status request");

    match JobStatusRepository::get_all_realm_job_status_summary(&service.pool).await {
        Ok(summaries) => {
            tracing::info!("Retrieved job status for all realms: {} entries", summaries.len());
            Ok(Json(summaries))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve all realms job status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /stats/jobs/counts - Get simple job counts by status
async fn job_counts_handler(
    State(service): State<ApiService>,
) -> Result<Json<HashMap<String, i64>>, StatusCode> {
    tracing::info!("Job counts request");

    match JobStatusRepository::get_job_counts_by_status(&service.pool).await {
        Ok(counts) => {
            tracing::info!("Retrieved job counts: {:?}", counts);
            Ok(Json(counts))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve job counts: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /admin/refresh-job-status - Manually trigger materialized view refresh
async fn refresh_job_status_handler(
    State(service): State<ApiService>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Manual job status refresh requested");

    match JobStatusService::force_refresh(&service.pool).await {
        Ok(_) => {
            tracing::info!("Job status materialized view refreshed successfully");
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Job status materialized view refreshed successfully",
                "timestamp": Utc::now()
            })))
        }
        Err(e) => {
            tracing::error!("Failed to refresh job status materialized view: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /checkpoint/calculate-rewards/:checkpoint_id - Calculate rewards for a checkpoint
pub async fn calculate_rewards_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<Vec<CheckpointRewardDistribution>>, StatusCode> {
    tracing::info!("Calculating rewards for checkpoint {}", checkpoint_id);

    let reward_service = CheckpointRewardService::new(service.pool.clone());

    match reward_service.calculate_and_distribute_rewards(checkpoint_id).await {
        Ok(distributions) => {
            tracing::info!("Successfully calculated {} reward distributions", distributions.len());
            Ok(Json(distributions))
        }
        Err(e) => {
            tracing::error!("Failed to calculate rewards: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/stats/:checkpoint_id - Get checkpoint statistics
pub async fn get_checkpoint_stats_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<CheckpointStats>, StatusCode> {
    match CheckpointStatsRepository::get_by_checkpoint_id(&service.pool, checkpoint_id).await {
        Ok(Some(stats)) => Ok(Json(stats)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get checkpoint stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/stats - Get checkpoint statistics by range
pub async fn get_checkpoint_stats_by_range_handler(
    State(service): State<ApiService>,
    Query(query): Query<CheckpointQuery>,
) -> Result<Json<Vec<CheckpointStats>>, StatusCode> {
    let start = query.start_checkpoint.unwrap_or(0);
    let end = query.end_checkpoint.unwrap_or(i64::MAX);

    match CheckpointStatsRepository::get_by_checkpoint_range(&service.pool, start, end).await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            tracing::error!("Failed to get checkpoint stats by range: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/summary/:checkpoint_id - Get checkpoint reward summary
pub async fn get_checkpoint_summary_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<CheckpointRewardSummary>, StatusCode> {
    let reward_service = CheckpointRewardService::new(service.pool.clone());

    match reward_service.get_checkpoint_summary(checkpoint_id).await {
        Ok(Some(summary)) => Ok(Json(summary)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get checkpoint summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/rewards/:worker_public_key - Get worker's reward aggregations
pub async fn get_worker_rewards_handler(
    State(service): State<ApiService>,
    Path(worker_public_key): Path<String>,
    Query(query): Query<WorkerRewardQuery>,
) -> Result<Json<WorkerRewardResponse>, StatusCode> {
    let time_period_str = query.time_period.unwrap_or_else(|| "1d".to_string());
    let limit = query.limit.unwrap_or(100);

    let time_period = match time_period_str.as_str() {
        "2m" => TimePeriod::TwoMinutes,
        "1h" => TimePeriod::OneHour,
        "1d" => TimePeriod::OneDay,
        "1w" => TimePeriod::OneWeek,
        "1m" => TimePeriod::OneMonth,
        _ => TimePeriod::OneDay,
    };

    let reward_service = CheckpointRewardService::new(service.pool.clone());

    match reward_service
        .get_worker_rewards_aggregated(
            &worker_public_key,
            time_period,
            query.start_time,
            query.end_time,
            limit,
        )
        .await
    {
        Ok(aggregations) => {
            let total_rewards: i64 = aggregations.iter().map(|a| a.total_rewards).sum();
            let total_jobs: i64 = aggregations.iter().map(|a| a.jobs_completed).sum();
            let total_checkpoints: i64 = aggregations.iter().map(|a| a.checkpoints_participated).sum();

            let response = WorkerRewardResponse {
                worker_public_key: worker_public_key.clone(),
                time_period: time_period_str,
                aggregations,
                total_rewards,
                total_jobs,
                total_checkpoints,
            };

            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to get worker rewards: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/rewards/:worker_public_key/stats - Get worker's overall reward statistics
pub async fn get_worker_reward_stats_handler(
    State(service): State<ApiService>,
    Path(worker_public_key): Path<String>,
) -> Result<Json<WorkerCheckpointRewardStats>, StatusCode> {
    let reward_service = CheckpointRewardService::new(service.pool.clone());

    match reward_service.get_worker_stats(&worker_public_key).await {
        Ok(Some(stats)) => Ok(Json(stats)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get worker reward stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}



/// GET /checkpoint/job-events/:checkpoint_id - Get job events for a checkpoint
pub async fn get_checkpoint_job_events_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<Vec<WorkerJobEvent>>, StatusCode> {
    match WorkerJobEventRepository::get_by_checkpoint(&service.pool, checkpoint_id).await {
        Ok(events) => Ok(Json(events)),
        Err(e) => {
            tracing::error!("Failed to get job events: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /checkpoint/distributions/:checkpoint_id - Get reward distributions for a checkpoint
pub async fn get_checkpoint_distributions_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<Vec<CheckpointRewardDistribution>>, StatusCode> {
    match CheckpointRewardDistributionRepository::get_by_checkpoint(&service.pool, checkpoint_id).await {
        Ok(distributions) => Ok(Json(distributions)),
        Err(e) => {
            tracing::error!("Failed to get reward distributions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}



async fn checkpoint_processing_status_handler(
    State(service): State<ApiService>,
) -> Result<Json<CheckpointProcessingStatus>, StatusCode> {
    let pending = match CheckpointRewardService::find_pending_checkpoints_public(&service.pool).await {
        Ok(checkpoints) => checkpoints,
        Err(e) => {
            tracing::error!("Failed to get pending checkpoints: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let last_processed = sqlx::query_scalar::<_, i64>(
        "SELECT MAX(checkpoint_id) FROM checkpoint_reward_distributions"
    )
        .fetch_optional(&service.pool)
        .await
        .ok()
        .flatten();

    let status = if pending.is_empty() {
        "All checkpoints processed".to_string()
    } else {
        format!("{} checkpoints pending", pending.len())
    };

    Ok(Json(CheckpointProcessingStatus {
        pending_count: pending.len(),
        pending_checkpoints: pending.iter().take(10).copied().collect(), // Show first 10
        last_processed_checkpoint: last_processed,
        status,
    }))
}
