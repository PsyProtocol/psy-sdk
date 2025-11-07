use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    models::{ContractResponse, ContractSummary, ListContractsParams, ListContractsResponse},
    repositories::contracts::ContractRepository,
    services::ApiService,
};

/// Get a contract by its ID with full details including function names
async fn get_contract_by_id_handler(State(service): State<ApiService>, Path(contract_id): Path<i64>) -> Result<Json<ContractResponse>, StatusCode> {
    match ContractRepository::get_by_id(&service.pool, contract_id).await {
        Ok(Some(contract)) => {
            info!("Contract retrieved: id={}, deployer={}", contract_id, contract.deployer);
            Ok(Json(contract))
        }
        Ok(None) => {
            info!("Contract not found: id={}", contract_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get contract {}: {}", contract_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a contract by its UUID with full details
async fn get_contract_by_uuid_handler(
    State(service): State<ApiService>,
    Path(contract_uuid): Path<Uuid>,
) -> Result<Json<ContractResponse>, StatusCode> {
    match ContractRepository::get_by_uuid(&service.pool, contract_uuid).await {
        Ok(Some(contract)) => {
            info!("Contract retrieved: uuid={}, id={}", contract_uuid, contract.contract_id);
            Ok(Json(contract))
        }
        Ok(None) => {
            info!("Contract not found: uuid={}", contract_uuid);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get contract {}: {}", contract_uuid, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List contracts with optional filtering by deployer, checkpoint, or function
/// name
async fn list_contracts_handler(
    State(service): State<ApiService>,
    Query(params): Query<ListContractsParams>,
) -> Result<Json<ListContractsResponse>, StatusCode> {
    // Validate pagination parameters
    let limit = params.limit.min(100).max(1);
    let offset = params.offset.max(0);

    // If searching by function name, use the specialized search
    if let Some(function_name) = params.function_name {
        match ContractRepository::search_by_function_name(&service.pool, &function_name, limit, offset).await {
            Ok(contracts) => {
                // Convert full responses to summaries for list view
                let summaries: Vec<ContractSummary> = contracts
                    .into_iter()
                    .map(|c| ContractSummary {
                        contract_id: c.contract_id,
                        contract_uuid: c.contract_uuid,
                        deployer: c.deployer,
                        checkpoint_id: c.checkpoint_id,
                        function_count: c.function_count,
                        timestamp: c.timestamp,
                    })
                    .collect();

                let total = summaries.len() as i64;

                return Ok(Json(ListContractsResponse {
                    contracts: summaries,
                    total,
                    limit,
                    offset,
                }));
            }
            Err(e) => {
                error!("Failed to search contracts by function name: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Regular list with optional filters
    let contracts = match ContractRepository::list(&service.pool, limit, offset, params.deployer.as_deref(), params.checkpoint_id).await {
        Ok(contracts) => contracts,
        Err(e) => {
            error!("Failed to list contracts: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get total count with same filters
    let total = match ContractRepository::count(&service.pool, params.deployer.as_deref(), params.checkpoint_id).await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to count contracts: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(ListContractsResponse {
        contracts,
        total,
        limit,
        offset,
    }))
}

/// Get contracts deployed by a specific address
async fn get_contracts_by_deployer_handler(
    State(service): State<ApiService>,
    Path(deployer): Path<String>,
    Query(params): Query<ListContractsParams>,
) -> Result<Json<ListContractsResponse>, StatusCode> {
    let limit = params.limit.min(100).max(1);
    let offset = params.offset.max(0);

    match ContractRepository::get_by_deployer(&service.pool, &deployer, limit, offset).await {
        Ok(contracts) => {
            let total = contracts.len() as i64;
            Ok(Json(ListContractsResponse {
                contracts,
                total,
                limit,
                offset,
            }))
        }
        Err(e) => {
            error!("Failed to get contracts for deployer {}: {}", deployer, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get contracts at a specific checkpoint
async fn get_contracts_by_checkpoint_handler(
    State(service): State<ApiService>,
    Path(checkpoint_id): Path<i64>,
) -> Result<Json<Vec<ContractSummary>>, StatusCode> {
    match ContractRepository::get_by_checkpoint(&service.pool, checkpoint_id).await {
        Ok(contracts) => {
            info!("Retrieved {} contracts at checkpoint {}", contracts.len(), checkpoint_id);
            Ok(Json(contracts))
        }
        Err(e) => {
            error!("Failed to get contracts for checkpoint {}: {}", checkpoint_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all unique function names across all contracts
async fn get_all_function_names_handler(State(service): State<ApiService>) -> Result<Json<Vec<String>>, StatusCode> {
    match ContractRepository::get_all_function_names(&service.pool).await {
        Ok(names) => {
            info!("Retrieved {} unique function names", names.len());
            Ok(Json(names))
        }
        Err(e) => {
            error!("Failed to get function names: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create the contracts router
pub fn create_contracts_router(api_service: ApiService) -> Router {
    Router::new()
        // List and search endpoints
        .route("/contracts", get(list_contracts_handler))
        .route("/contracts/functions", get(get_all_function_names_handler))
        // Get by specific identifiers
        .route("/contracts/id/{contract_id}", get(get_contract_by_id_handler))
        .route("/contracts/uuid/{contract_uuid}", get(get_contract_by_uuid_handler))
        // Get by relationships
        .route("/contracts/deployer/{deployer}", get(get_contracts_by_deployer_handler))
        .route("/contracts/checkpoint/{checkpoint_id}", get(get_contracts_by_checkpoint_handler))
        .with_state(api_service)
}
