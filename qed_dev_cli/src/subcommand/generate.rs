use anyhow::{Context, Result};
use minijinja::{Environment, context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, Child};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error, debug};
use tokio::time::{sleep, Duration};

use crate::aws::{
    SimpleInstanceSelector, SimpleInstanceRecommendation,
};

use super::{GenerateArgs, GenerateCommands, RunArgs, GenerateDockerComposeArgs, GenerateAwsArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub nodes: NodesConfig,
    pub global_api_services: Option<GlobalApiServices>
}

pub use qed_prover::local::provider::{NetworkConfig, RealmConfig, CoordinatorConfig};
use crate::subcommand::launch::ensure_worker_config_file;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesConfig {
    pub coordinator: CoordinatorNode,
    pub realms: Vec<RealmNode>,
    pub prover: Option<ServiceConfig>,
    pub deployment: DeploymentConfig,
    pub workers: Option<IndependentWorkers>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndependentWorkers {
    pub enabled: bool,
    pub worker_pools: Vec<WorkerPool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPool {
    pub id: String,
    pub instances: u32,
    pub args: HashMap<String, Value>,
    pub env: HashMap<String, String>,
    pub aws: Option<AwsServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerGlobalConfig {
    pub task_discovery_interval: u32,
    pub max_concurrent_tasks: u32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendConfig {
    #[serde(rename = "type")]
    pub database: String,
    pub lmdbx: Option<LmdbxConfig>,
    pub scylla: Option<ScyllaConfig>,
    pub tikv: Option<TikvConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LmdbxConfig {
    pub path: String,
    pub mmap_size_gb: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<LmdbxAwsConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LmdbxAwsConfig {
    pub volume_size: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScyllaConfig {
    pub endpoints: Vec<String>,
    pub replication_factor: u32,
    pub consistency_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<ScyllaAwsConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScyllaAwsConfig {
    pub cpu: u32,
    pub memory: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_type: Option<DeploymentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec2: Option<Ec2Config>,
    #[serde(default = "default_data_volume_size")]
    pub data_volume_size: u32,
    #[serde(default = "default_commitlog_volume_size")]
    pub commitlog_volume_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikvConfig {
    pub pd_endpoints: Vec<String>,
    pub namespace: String,
    pub aws: Option<TikvAwsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikvAwsConfig {
    pub cpu: u32,
    pub memory: u32,
    pub deployment_type: Option<DeploymentType>,
    pub ec2: Option<Ec2ServiceConfig>,
    #[serde(default = "default_data_volume_size")]
    pub data_volume_size: u32,
    #[serde(default = "default_pd_count")]
    pub pd_count: u32,
    #[serde(default = "default_tikv_count")]
    pub tikv_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub coordinator: RedisInstance,
    pub realms: Vec<RedisRealm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisInstance {
    pub uri: String,
    pub pool_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<RedisAwsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisAwsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_type: Option<RedisDeploymentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_size: Option<u32>,
    // ElastiCache configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elasticache: Option<ElastiCacheConfig>,
    // EC2 configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec2: Option<RedisEc2Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedisDeploymentType {
    ElastiCache,
    EC2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElastiCacheConfig {
    pub node_type: String,
    pub num_cache_nodes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisEc2Config {
    pub min_instances: u32,
    pub max_instances: u32,
    pub desired_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisRealm {
    pub id: u64,
    pub uri: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorNode {
    pub id: u64,
    pub redis: Option<RedisInstance>,
    pub backend: Option<BackendConfig>,
    pub processor: ServiceConfig,
    pub edge: ServiceConfig,
    pub watcher: Option<ServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmNode {
    pub id: u64,
    pub redis: Option<RedisInstance>,
    pub backend: Option<BackendConfig>,
    pub processor: ServiceConfig,
    pub edge: ServiceConfig,
    pub watcher: Option<ServiceConfig>,
}
// Add new top-level service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalApiServices {
    pub api_service: Option<ServiceConfig>,
    pub timescaledb: Option<TimescaleDbConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimescaleDbConfig {
    pub enabled: bool,
    pub connection_string: String,
    pub aws: Option<RdsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdsConfig {
    pub instance_class: String,
    pub allocated_storage: u32,
    pub multi_az: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances: Option<u32>,
    pub args: HashMap<String, Value>,
    pub env: HashMap<String, String>,
    pub aws: Option<AwsServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsServiceConfig {
    pub cpu: u32,
    pub memory: u32,
    pub deployment_type: Option<DeploymentType>,
    pub load_balancer: Option<LoadBalancerConfig>,
    // EC2-specific config
    pub ec2: Option<Ec2ServiceConfig>,
    // ECS-specific config
    pub ecs: Option<EcsServiceConfig>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub port: u16,
    pub health_check_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentType {
    #[serde(rename = "ecs")]
    ECS,
    #[serde(rename = "ec2")]
    EC2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2ServiceConfig {
    pub instance_type: Option<String>, // Optional, can be auto-calculated
    pub min_instances: u32,
    pub max_instances: u32,
    pub desired_instances: u32,
    pub security_groups: Option<Vec<String>>,
    pub iam_role: Option<String>,
    pub user_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsServiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances: Option<u32>, // For local deployment
    pub task_count: u32, // For AWS ECS deployment
    pub deployment_configuration: Option<EcsDeploymentConfig>,
    pub network_mode: Option<String>, // awsvpc, bridge, host
    pub placement_strategy: Option<Vec<PlacementStrategy>>,
    pub placement_constraints: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsDeploymentConfig {
    pub maximum_percent: Option<u32>,
    pub minimum_healthy_percent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementStrategy {
    pub r#type: String, // spread, binpack, random
    pub field: Option<String>, // instanceId, attribute:ecs.availability-zone
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub aws: AwsDeploymentConfig,
    pub docker: DockerDeploymentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsDeploymentConfig {
    pub region: String,
    pub project_name: String,
    pub vpc: VpcConfig,
    pub ecs: EcsConfig,
    pub ecr: EcrConfig,
    pub s3: S3Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpcConfig {
    pub cidr: String,
    pub availability_zones: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsConfig {
    pub cluster_name: String,
    pub log_group: String,
    pub ec2: Option<Ec2Config>,
    pub fargate: Option<FargateConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ec2Config {
    pub instance_type: Option<String>,
    pub min_instances: u32,
    pub max_instances: u32,
    pub desired_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FargateConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcrConfig {
    pub repository_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerDeploymentConfig {
    pub network_name: String,
    pub network_subnet: String,
}

pub async fn run(args: GenerateArgs) -> Result<()> {
    // Load config
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file: {}", args.config))?;
    let config: Config = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse config.json")?;

    match args.command {
        GenerateCommands::DockerCompose(compose_args) => generate_docker_compose(&config, compose_args).await,
        GenerateCommands::Aws(aws_args) => generate_aws_templates(&config, aws_args).await,
    }
}

pub async fn run_deployment(args: RunArgs) -> Result<()> {
    // Load config
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file: {}", args.config))?;
    let config: Config = serde_json::from_str(&config_content)
        .with_context(|| "Failed to parse config.json")?;

    if args.stop {
        return stop_deployment().await;
    }

    run_deployment_impl(&config, args).await
}

async fn run_deployment_impl(config: &Config, args: RunArgs) -> Result<()> {
    info!("Starting QED network deployment...");

    // Override backend type if specified
    let database = args.backend.as_ref()
        .map(|s| s.as_str())
        .unwrap_or_else(|| {
            // Try to get from coordinator backend, fallback to "lmdbx" if no global config
            config.nodes.coordinator.backend.as_ref()
                .map(|b| b.database.as_str())
                .unwrap_or("lmdbx")
        });

    // Create necessary directories based on backend type
    match database {
        "lmdbx" => {
            // Create directories for LMDBX
            if let Some(coord_backend) = &config.nodes.coordinator.backend {
                if let Some(lmdbx_config) = &coord_backend.lmdbx {
                    fs::create_dir_all(&lmdbx_config.path)
                        .with_context(|| format!("Failed to create directory: {}", lmdbx_config.path))?;
                }
            }
            for realm in &config.nodes.realms {
                if let Some(realm_backend) = &realm.backend {
                    if let Some(lmdbx_config) = &realm_backend.lmdbx {
                        fs::create_dir_all(&lmdbx_config.path)
                            .with_context(|| format!("Failed to create directory: {}", lmdbx_config.path))?;
                    }
                }
            }
        },
        "scylla" => {
            // Start ScyllaDB
            start_scylladb(config)?;
            // Wait for ScyllaDB to be ready
            std::thread::sleep(std::time::Duration::from_secs(30));
        },
        "tikv" => {
            // Start TiKV cluster
            start_tikv_cluster(config)?;
            // Wait for TiKV to be ready
            std::thread::sleep(std::time::Duration::from_secs(15));
        },
        _ => {
            return Err(anyhow::anyhow!("Unsupported database backend: {}", database));
        }
    }

    // Start Redis instances
    start_redis_instances(config)?;

    // Start TimescaleDB if configured
    if let Some(global_services) = &config.global_api_services {
        // Start TimescaleDB first if enabled
        if let Some(timescale_config) = &global_services.timescaledb {
            if timescale_config.enabled {
                info!("Starting TimescaleDB for API service...");
                start_timescaledb()?;
                // Wait for DB to be ready
                sleep(std::time::Duration::from_secs(10)).await;
                // Run migrations

                run_database_migrations()?;
            }
        }

        // Start API Service
        if let Some(api_config) = &global_services.api_service {
            if api_config.enabled {
                info!("Starting API service on port 3000...");
                start_service(
                    "qed-api-service",
                    build_api_service_command(api_config)?,
                    &api_config.env,
                )?;

                // Wait for API service to be ready
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
    }


    // Start coordinator services
    start_coordinator_services(config, database, &args)?;

    // Start realm services
    start_realm_services(config, database, &args)?;

    start_independent_workers(config, &args)?;
    info!("✅ QED network deployment started successfully!");

    // Print deployment summary
    print_deployment_summary2(config, database);


    if !args.detach {
        info!("Press Ctrl+C to stop the deployment...");
        tokio::signal::ctrl_c().await?;
        stop_deployment().await?;
    }

    Ok(())
}

fn build_api_service_command(service_config: &ServiceConfig) -> Result<Vec<String>> {
    let mut cmd = vec![
        "./target/release/qed_api_services".to_string(),
    ];

    // Add service-specific args if any
    for (key, value) in &service_config.args {
        cmd.push(format!("--{}", key.replace('_', "-")));
        match value {
            Value::String(s) => cmd.push(s.clone()),
            Value::Number(n) => cmd.push(n.to_string()),
            Value::Bool(b) => if *b { cmd.push("true".to_string()); } else { cmd.push("false".to_string()); },
            _ => {}
        }
    }

    Ok(cmd)
}
fn start_redis_instances(config: &Config) -> Result<()> {
    info!("Starting Redis instances...");

    // Extract port from coordinator URI
    let coordinator_redis = config.nodes.coordinator.redis.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Redis configuration found for coordinator"))?;
    let coordinator_port = extract_port(&coordinator_redis.uri)?;
    start_redis_instance(coordinator_port, "coordinator")?;

    // Start realm Redis instances
    for realm in &config.nodes.realms {
        if let Some(realm_redis) = &realm.redis {
            let port = extract_port(&realm_redis.uri)?;
            start_redis_instance(port, &format!("realm_{}", realm.id))?;
        }
    }

    // Wait for Redis to be ready
    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok(())
}

fn extract_port(uri: &str) -> Result<u16> {
    let parts: Vec<&str> = uri.split(':').collect();
    if parts.len() >= 3 {
        parts[2].parse::<u16>()
            .with_context(|| format!("Failed to parse port from URI: {}", uri))
    } else {
        Err(anyhow::anyhow!("Invalid Redis URI format: {}", uri))
    }
}

fn start_redis_instance(port: u16, name: &str) -> Result<()> {
    info!("Starting Redis instance '{}' on port {}", name, port);

    let output = Command::new("docker")
        .args(&[
            "run",
            "-d",
            "--name", &format!("qed-redis-{}", name),
            "-p", &format!("{}:{}", port, port),
            "redis:7-alpine"
        ])
        .output()
        .context("Failed to start Redis container")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already in use") {
            warn!("Redis container {} already exists, skipping...", name);
        } else {
            return Err(anyhow::anyhow!("Failed to start Redis: {}", stderr));
        }
    }

    Ok(())
}

fn start_scylladb(config: &Config) -> Result<()> {
    info!("Starting ScyllaDB...");

    // This is a simplified version - in production, you'd use the AWS deployment
    let output = Command::new("docker")
        .args(&[
            "run",
            "-d",
            "--name", "qed-scylladb",
            "-p", "9042:9042",
            "scylladb/scylla:2025.1",
            "--smp", "2",
            "--memory", "4G"
        ])
        .output()
        .context("Failed to start ScyllaDB container")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already in use") {
            warn!("ScyllaDB container already exists, skipping...");
        } else {
            return Err(anyhow::anyhow!("Failed to start ScyllaDB: {}", stderr));
        }
    }

    // Wait for ScyllaDB to be ready
    info!("Waiting for ScyllaDB to be ready...");
    std::thread::sleep(std::time::Duration::from_secs(30));

    // Create keyspaces
    create_scylla_keyspaces(config)?;

    Ok(())
}

fn create_scylla_keyspaces(config: &Config) -> Result<()> {
    let keyspaces = vec![
        "qed_coordinator",
        "qed_realm_0",
        "qed_realm_1",
    ];

    for keyspace in keyspaces {
        info!("Creating keyspace: {}", keyspace);
        let output = Command::new("docker")
            .args(&[
                "exec",
                "qed-scylladb",
                "cqlsh",
                "-e",
                &format!(
                    "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': 1}};",
                    keyspace
                )
            ])
            .output()
            .context("Failed to create keyspace")?;

        if !output.status.success() {
            warn!("Failed to create keyspace {}: {}", keyspace, String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

fn start_coordinator_services(config: &Config, database: &str, args: &RunArgs) -> Result<()> {
    info!("Starting coordinator services...");

    let coordinator_redis = config.nodes.coordinator.redis.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Redis configuration found for coordinator"))?;
    let redis_uri = &coordinator_redis.uri;
    let node = &config.nodes.coordinator;

    // Use node-specific backend
    let backend_config = node.backend.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No backend configuration found for coordinator"))?;
    let node_database = &backend_config.database;

    // Start processor
    if node.processor.enabled {
        start_service(
            "coordinator-processor",
            build_service_command(
                "coordinator-processor",
                node_database,
                redis_uri,
                None,
                &node.processor,
                config,
                Some(backend_config),
            )?,
            &node.processor.env,
        )?;
    }

    if let Some(watcher_config) = &node.watcher {
        if watcher_config.enabled {
            start_service(
                "coordinator-watcher",
                build_watcher_command("coordinator", 0, redis_uri, watcher_config, config, Some(backend_config))?,
                &watcher_config.env,
            )?;
        }
    }
    // // Start workers
    // if node.worker.enabled {
    //     let instances = node.worker.aws.as_ref()
    //         .and_then(|aws| match &aws.deployment_type {
    //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances),
    //             _ => aws.ecs.as_ref().and_then(|e| e.instances.or(Some(e.task_count)))
    //         })
    //         .or(node.worker.instances)
    //         .unwrap_or(1);
    //     for i in 0..instances {
    //         start_service(
    //             &format!("coordinator-worker-{}", i),
    //             build_service_command(
    //                 "coordinator-worker",
    //                 node_database,
    //                 redis_uri,
    //                 None,
    //                 &node.worker,
    //                 config,
    //                     Some(backend_config),
    //             )?,
    //             &node.worker.env,
    //         )?;
    //     }
    // }

    // Wait a bit before starting edge
    // std::thread::sleep(std::time::Duration::from_secs(2));

    // Start edge
    if node.edge.enabled {
        start_service(
            "coordinator-edge",
            build_service_command(
                "coordinator-edge",
                node_database,
                redis_uri,
                None,
                &node.edge,
                config,
                Some(backend_config),
            )?,
            &node.edge.env,
        )?;
    }

    Ok(())
}

fn start_realm_services(config: &Config, database: &str, args: &RunArgs) -> Result<()> {
    info!("Starting realm services...");

    for realm_node in &config.nodes.realms {
        let realm_redis = realm_node.redis.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Redis config not found for realm {}", realm_node.id))?;

        let redis_uri = &realm_redis.uri;

        // Use realm-specific backend
        let backend_config = realm_node.backend.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No backend configuration found for realm {}", realm_node.id))?;
        let realm_database = &backend_config.database;

        info!("Starting services for realm {} with {} backend...", realm_node.id, realm_database);

        // Start processor
        if realm_node.processor.enabled {
            start_service(
                &format!("realm-{}-processor", realm_node.id),
                build_service_command(
                    "realm-processor",
                    realm_database,
                    redis_uri,
                    Some(realm_node),
                    &realm_node.processor,
                    config,
                        Some(backend_config),
                )?,
                &realm_node.processor.env,
            )?;
        }

        // Wait a bit before starting edge
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Start edge
        if realm_node.edge.enabled {
            start_service(
                &format!("realm-{}-edge", realm_node.id),
                build_service_command(
                    "realm-edge",
                    realm_database,
                    redis_uri,
                    Some(realm_node),
                    &realm_node.edge,
                    config,
                        Some(backend_config),
                )?,
                &realm_node.edge.env,
            )?;
        }

        if let Some(watcher_config) = &realm_node.watcher {
            if watcher_config.enabled {
                start_service(
                    &format!("realm-{}-watcher", realm_node.id),
                    build_watcher_command("realm", realm_node.id as u32, redis_uri, watcher_config, config, Some(backend_config))?,
                    &watcher_config.env,
                )?;
            }
        }
    }

    Ok(())
}
fn start_independent_workers(config: &Config, _args: &RunArgs) -> Result<()> {
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            info!("Starting independent worker pools...");
            // Ensure config.json exists with correct RPC URLs for workers
            // ensure_worker_config_file(config)?;

            for pool in &workers.worker_pools {
                info!("Starting worker pool '{}' with {} instances", pool.id, pool.instances);
                for i in 0..pool.instances {
                    let service_name = format!("worker-{}-{}", pool.id, i);
                    let command = build_worker_command(pool)?;
                    // Add API service URL to environment
                    let mut env = pool.env.clone();
                    env.insert("API_SERVICE_URL".to_string(), "http://localhost:3000".to_string());

                    start_service(&service_name, command, &env)?;                }
            }
        }
    }
    Ok(())
}
fn build_worker_command(pool: &WorkerPool) -> Result<Vec<String>> {
    let mut cmd = vec![
        "./target/release/qed_rollup_cli".to_string(),
        "worker".to_string(),
    ];
    // Add pool-specific args (private-key, keystore-path, wallet-password)
    for (key, value) in &pool.args {
        // Skip config if it's in args since we already added it
        if key == "config" {
            continue;
        }

        cmd.push(format!("--{}", key.replace('_', "-")));

        match value {
            Value::String(s) => cmd.push(s.clone()),
            Value::Number(n) => cmd.push(n.to_string()),
            Value::Bool(b) => {
                if *b {
                    cmd.push("true".to_string());
                } else {
                    cmd.push("false".to_string());
                }
            },
            _ => {}
        }
    }

    Ok(cmd)
}

fn build_watcher_command(
    node_type: &str,
    node_id: u32,
    redis_uri: &str,
    service_config: &ServiceConfig,
    config: &Config,
    backend_config: Option<&BackendConfig>,
) -> Result<Vec<String>> {
    let mut cmd = vec![
        "./target/release/qed_rollup_cli".to_string(),
        "watcher".to_string(),
    ];

    // Add node type
    cmd.push("--node-type".to_string());
    cmd.push(node_type.to_string());

    cmd.push("--node-id".to_string());
    cmd.push(node_id.to_string());

    // Add database config (TiKV)
    if let Some(backend) = backend_config {
        if let Some(tikv_config) = &backend.tikv {
            cmd.push("--database".to_string());
            cmd.push("tikv".to_string());

            cmd.push("--tikv-pd-endpoints".to_string());
            cmd.push(tikv_config.pd_endpoints.join(","));

            cmd.push("--tikv-namespace".to_string());
            cmd.push(format!("watcher-{}", node_type));
        }
    }
    // Add Redis URI
    cmd.push("--redis-url".to_string());
    cmd.push(redis_uri.to_string());

    // Add API endpoint
    cmd.push("--api-endpoint".to_string());
    cmd.push("http://localhost:3000".to_string()); // Default API service endpoint

    Ok(cmd)
}
fn build_service_command(
    service_type: &str,
    database: &str,
    redis_uri: &str,
    realm_node: Option<&RealmNode>,
    service_config: &ServiceConfig,
    config: &Config,
    backend_config: Option<&BackendConfig>,
) -> Result<Vec<String>> {
    let mut cmd = vec![
        "./target/release/qed_rollup_cli".to_string(),
        service_type.to_string(),
    ];

    // Add backend-specific args
    let backend = backend_config.ok_or_else(|| anyhow::anyhow!("Backend configuration is required"))?;

    if backend.database != "tikv" {
        return Err(anyhow::anyhow!("Only TiKV backend is supported now"));
    }

    match database {
        "lmdbx" => {
            cmd.push("--database".to_string());
            cmd.push("lmdbx".to_string());

            if let Some(lmdbx_config) = &backend.lmdbx {
                let path = if let Some(realm) = realm_node {
                    format!("{}/realm{}", lmdbx_config.path, realm.id)
                } else {
                    format!("{}/coordinator", lmdbx_config.path)
                };
                cmd.push("--lmdbx-path".to_string());
                cmd.push(path);
            }
        }
        "scylla" => {
            cmd.push("--database".to_string());
            cmd.push("scylla".to_string());

            if let Some(scylla_config) = &backend.scylla {
                cmd.push("--scylla-uri".to_string());
                cmd.push(scylla_config.endpoints[0].clone());

                let keyspace = if let Some(realm) = realm_node {
                    format!("qed_realm_{}", realm.id)
                } else {
                    "qed_coordinator".to_string()
                };
                cmd.push("--scylla-keyspace".to_string());
                cmd.push(keyspace);
            }
        }
        "tikv" => {
            cmd.push("--database".to_string());
            cmd.push("tikv".to_string());

            if let Some(tikv_config) = &backend.tikv {
                cmd.push("--tikv-pd-endpoints".to_string());
                cmd.push(tikv_config.pd_endpoints.join(","));

                let namespace = if let Some(realm) = realm_node {
                    format!("{}-realm{}", tikv_config.namespace, realm.id)
                } else {
                    format!("{}-coordinator", tikv_config.namespace)
                };
                cmd.push("--tikv-namespace".to_string());
                cmd.push(namespace);
            }
        }
        _ => return Err(anyhow::anyhow!("Unknown backend type: {}", database)),
    }

    // Add Redis URI
    cmd.push("--redis-uri".to_string());
    cmd.push(redis_uri.to_string());

    // Add realm-specific args
    if let Some(realm) = realm_node {
        cmd.push("--realm-id".to_string());
        cmd.push(realm.id.to_string());
    }

    // Add service-specific args
    for (key, value) in &service_config.args {
        cmd.push(format!("--{}", key.replace('_', "-")));
        match value {
            Value::String(s) => cmd.push(s.clone()),
            Value::Number(n) => cmd.push(n.to_string()),
            Value::Bool(b) => if *b { cmd.push("true".to_string()); } else { cmd.push("false".to_string()); },
            _ => {}
        }
    }


    Ok(cmd)
}

fn add_database_config(
    cmd: &mut Vec<String>,
    database: &str,
    backend_config: Option<&BackendConfig>,
    realm_node: Option<&RealmNode>,
) -> Result<()> {
    let backend = backend_config.ok_or_else(|| anyhow::anyhow!("Backend configuration is required"))?;

    match database {
        "lmdbx" => {
            cmd.push("--database".to_string());
            cmd.push("lmdbx".to_string());

            if let Some(lmdbx_config) = &backend.lmdbx {
                let path = if let Some(realm) = realm_node {
                    format!("{}/realm{}", lmdbx_config.path, realm.id)
                } else {
                    format!("{}/coordinator", lmdbx_config.path)
                };
                cmd.push("--lmdbx-path".to_string());
                cmd.push(path);
            }
        }
        "scylla" => {
            cmd.push("--database".to_string());
            cmd.push("scylla".to_string());

            if let Some(scylla_config) = &backend.scylla {
                cmd.push("--scylla-uri".to_string());
                cmd.push(scylla_config.endpoints[0].clone());

                let keyspace = if let Some(realm) = realm_node {
                    format!("qed_realm_{}", realm.id)
                } else {
                    "qed_coordinator".to_string()
                };
                cmd.push("--scylla-keyspace".to_string());
                cmd.push(keyspace);
            }
        }
        "tikv" => {
            cmd.push("--database".to_string());
            cmd.push("tikv".to_string());

            if let Some(tikv_config) = &backend.tikv {
                cmd.push("--tikv-pd-endpoints".to_string());
                cmd.push(tikv_config.pd_endpoints.join(","));

                let namespace = if let Some(realm) = realm_node {
                    format!("{}-realm{}", tikv_config.namespace, realm.id)
                } else {
                    format!("{}-coordinator", tikv_config.namespace)
                };
                cmd.push("--tikv-namespace".to_string());
                cmd.push(namespace);
            }
        }
        _ => return Err(anyhow::anyhow!("Unknown backend type: {}", database)),
    }

    Ok(())
}
// TiKV cluster startup function
fn start_tikv_cluster(config: &Config) -> Result<()> {
    info!("Starting TiKV cluster...");

    // Check if docker-compose file exists
    let tikv_compose_file = "./scripts/docker-compose.tikv.yml";
    if !Path::new(tikv_compose_file).exists() {
        return Err(anyhow::anyhow!(
            "TiKV docker-compose file not found at {}. Please ensure the file exists.",
            tikv_compose_file
        ));
    }

    // Start TiKV using docker-compose
    let output = Command::new("docker-compose")
        .args(&["-f", tikv_compose_file, "up", "-d"])
        .output()
        .context("Failed to start TiKV cluster")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") || stderr.contains("already in use") {
            warn!("TiKV cluster may already be running, continuing...");
        } else {
            return Err(anyhow::anyhow!("Failed to start TiKV cluster: {}", stderr));
        }
    }

    info!("Waiting for TiKV cluster to be ready...");

    // Wait and verify TiKV is ready by checking PD health
    for i in 1..=30 {
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Try to connect to PD (Placement Driver)
        let pd_client_check = Command::new("curl")
            .args(&["-s", "http://localhost:2379/health"])
            .output();

        if let Ok(output) = pd_client_check {
            if output.status.success() {
                info!("TiKV PD cluster is ready!");
                return Ok(());
            }
        }

        if i % 5 == 0 {
            info!("Still waiting for TiKV cluster... ({}/30)", i);
        }
    }

    Err(anyhow::anyhow!("TiKV cluster failed to start within timeout"))
}
fn start_timescaledb() -> Result<()> {
    info!("Starting TimescaleDB...");
    let output = Command::new("docker")
        .args(&[
            "run", "-d", "--name", "qed-timescaledb",
            "-p", "5432:5432",
            "-e", "POSTGRES_PASSWORD=password",
            "-e", "POSTGRES_DB=qed",
            "timescale/timescaledb:latest-pg17"
        ])
        .output()
        .context("Failed to start TimescaleDB")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already in use") {
            warn!("TimescaleDB container already exists, checking if it's running...");

            // Check if container is running
            let check = Command::new("docker")
                .args(&["ps", "-q", "-f", "name=qed-timescaledb"])
                .output()?;

            if check.stdout.is_empty() {
                // Container exists but not running, start it
                Command::new("docker")
                    .args(&["start", "qed-timescaledb"])
                    .output()?;
            }
        } else {
            return Err(anyhow::anyhow!("Failed to start TimescaleDB: {}", stderr));
        }
    }

    // Wait for database to be ready and run migrations
    std::thread::sleep(std::time::Duration::from_secs(10));
    run_database_migrations()?;
    info!("TimescaleDB started successfully");

    Ok(())
}
// Database migration function
fn run_database_migrations() -> Result<()> {
    info!("Running database migrations...");

    // First check if qed_api_services directory exists
    if !Path::new("./qed_api_services").exists() {
        warn!("qed_api_services directory not found, skipping migrations");
        return Ok(());
    }

    let output = Command::new("cargo")
        .args(&["sqlx", "migrate", "run"])
        .current_dir("./qed_api_services")
        .env("DATABASE_URL", "postgres://postgres:password@localhost:5432/qed")
        .output()
        .context("Failed to run database migrations")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if it's just "no migrations to run"
        if stderr.contains("No migrations") || stderr.contains("up to date") {
            info!("Database is already up to date");
        } else {
            warn!("Migration output: {}", stderr);
        }
    } else {
        info!("Database migrations completed successfully");
    }

    Ok(())
}
fn start_service(name: &str, command: Vec<String>, env: &HashMap<String, String>) -> Result<()> {
    info!("Starting service: {}", name);

    let log_file = format!("logs/{}.log", name);
    fs::create_dir_all("logs")?;

    // Load .env file and merge with config env vars
    let mut merged_env = HashMap::new();

    // First, load environment variables from .env file
    // dotenv::vars() returns an iterator of (key, value) pairs from .env file
    for (key, value) in dotenv::vars() {
        merged_env.insert(key, value);
    }

    // Then override with config.json env vars (higher priority)
    for (key, value) in env {
        merged_env.insert(key.clone(), value.clone());
    }

    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .envs(&merged_env)
        .stdout(Stdio::from(fs::File::create(&log_file)?))
        .stderr(Stdio::from(fs::File::create(format!("logs/{}.err", name))?))
        .spawn()
        .with_context(|| format!("Failed to start service: {}", name))?;

    Ok(())
}

async fn stop_deployment() -> Result<()> {
    info!("Stopping QED network deployment...");

    // Stop all QED services
    let output = Command::new("pkill")
        .args(&["-f", "qed_rollup_cli"])
        .output()
        .context("Failed to stop QED services")?;

    if !output.status.success() {
        warn!("No QED services found to stop");
    }

    // Stop Docker containers
    let containers = vec!["qed-redis-coordinator", "qed-redis-realm_0", "qed-redis-realm_1", "qed-scylladb"];
    for container in containers {
        let _ = Command::new("docker")
            .args(&["stop", container])
            .output();
        let _ = Command::new("docker")
            .args(&["rm", container])
            .output();
    }

    info!("QED network deployment stopped");
    Ok(())
}

async fn generate_docker_compose(config: &Config, args: GenerateDockerComposeArgs) -> Result<()> {
    info!("Generating docker-compose.yml...");

    let template = include_str!("../../../.github/templates/docker/docker-compose.yml.j2");
    let mut env = Environment::new();
    env.add_template("docker-compose", template)?;

    // Use simplified instance selector to calculate recommendations
    let selector = SimpleInstanceSelector::new();
    let service_requirements = SimpleInstanceSelector::build_service_requirements_from_config(config);

    let recommendations = if !service_requirements.is_empty() {
        match selector.calculate_multiple_recommendations(service_requirements) {
            Ok(recs) => recs,
            Err(e) => {
                warn!("Failed to calculate instance recommendations: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let tmpl = env.get_template("docker-compose")?;
    let output = tmpl.render(context! {
        config => config,
        recommendations => &recommendations,
    })?;

    fs::write(&args.output, output)
        .with_context(|| format!("Failed to write docker-compose.yml to {}", args.output))?;

    info!("Generated docker-compose.yml at {}", args.output);

    // Print Docker Compose deployment summary (simplified version, no AWS info)
    print_docker_compose_summary(config)?;

    Ok(())
}

fn print_deployment_summary(config: &Config, recommendations: &[SimpleInstanceRecommendation]) -> Result<()> {
    info!("\n📊 QED Network Deployment Summary");
    info!("┌─────────────────┬──────────────────────────────────┬──────────┬──────────────┬───────────────────────┬──────────────────────────┐");
    info!("│ Component       │ Services (Proc/Edge/Worker)      │ Total CPU│ Total Memory │ Instance Count        │ Instance Types           │");
    info!("├─────────────────┼──────────────────────────────────┼──────────┼──────────────┼───────────────────────┼──────────────────────────┤");

    // Create recommendation mapping
    let mut recommendation_map = std::collections::HashMap::new();
    for rec in recommendations {
        recommendation_map.insert(rec.group_name.clone(), rec);
    }

    // Coordinator section
    let coord_backend = config.nodes.coordinator.backend.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No backend configuration found for coordinator"))?;
    let coord_storage = format_storage_info(coord_backend, Some("coordinator"));

    let mut coord_services = Vec::new();
    let mut coord_total_cpu = 0;
    let mut coord_total_memory = 0;

    if config.nodes.coordinator.processor.enabled {
        if let Some(aws) = &config.nodes.coordinator.processor.aws {
            let task_count = match &aws.deployment_type {
                Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
                Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                _ => 1
            };
            coord_services.push(format!("Processor({})", task_count));
            coord_total_cpu += aws.cpu * task_count;
            coord_total_memory += aws.memory * task_count;
        }
    }

    if config.nodes.coordinator.edge.enabled {
        if let Some(aws) = &config.nodes.coordinator.edge.aws {
            let task_count = match &aws.deployment_type {
                Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
                Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                _ => 1
            };
            coord_services.push(format!("Edge({})", task_count));
            coord_total_cpu += aws.cpu * task_count;
            coord_total_memory += aws.memory * task_count;
        }
    }

    // if config.nodes.coordinator.worker.enabled {
    //     if let Some(aws) = &config.nodes.coordinator.worker.aws {
    //         let task_count = match &aws.deployment_type {
    //             Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
    //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
    //             _ => 1
    //         };
    //         coord_services.push(format!("Worker({})", task_count));
    //         coord_total_cpu += aws.cpu * task_count;
    //         coord_total_memory += aws.memory * task_count;
    //     }
    // }

    let coord_config_info = format!("Redis :6379, Edge :8545");

    // Build instance details for each service
    let mut coord_instance_details = Vec::new();
    if config.nodes.coordinator.processor.enabled {
        let rec = get_service_instance_recommendation(&config.nodes.coordinator.processor, "processor");
        coord_instance_details.push(format!("Processor: {}", rec));
    }
    if config.nodes.coordinator.edge.enabled {
        let rec = get_service_instance_recommendation(&config.nodes.coordinator.edge, "edge");
        coord_instance_details.push(format!("Edge: {}", rec));
    }
    // if config.nodes.coordinator.worker.enabled {
    //     let rec = get_service_instance_recommendation(&config.nodes.coordinator.worker, "worker");
    //     coord_instance_details.push(format!("Worker: {}", rec));
    // }

    if !coord_services.is_empty() {
        // Build short instance summary for the main row
        let instance_summary = if coord_instance_details.len() > 0 {
            format!("{} instances", coord_instance_details.len())
        } else {
            "No instances".to_string()
        };

        // Count total EC2 instances (not tasks)
        let mut total_instances = 0;
        if config.nodes.coordinator.processor.enabled {
            if let Some(aws) = &config.nodes.coordinator.processor.aws {
                total_instances += match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => 1 // For ECS, assume 1 instance per task for now
                };
            }
        }
        if config.nodes.coordinator.edge.enabled {
            if let Some(aws) = &config.nodes.coordinator.edge.aws {
                total_instances += match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                };
            }
        }
        // if config.nodes.coordinator.worker.enabled {
        //     if let Some(aws) = &config.nodes.coordinator.worker.aws {
        //         total_instances += match &aws.deployment_type {
        //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
        //             _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
        //         };
        //     }
        // }

        info!("│ {:<15} │ {:<32} │ {:<8} │ {:<12} │ {:<21} │                          │",
            "Coordinator",
            coord_services.join("+"),
            format!("{:.1}", coord_total_cpu as f32 / 1024.0),
            format!("{:.0}GB", coord_total_memory as f32 / 1024.0),
            format!("{} total", total_instances)
        );

        // Display instance details on separate lines
        for detail in coord_instance_details {
            info!("│                 │                                  │          │              │                       │ {:<24} │", detail);
        }

        info!("│                 │ {:<32} │          │              │                       │                          │",
            format!("Storage: {}", coord_storage)
        );
        info!("│                 │ {:<32} │          │              │                       │                          │",
            format!("Network: {}", coord_config_info)
        );
        info!("├─────────────────┼──────────────────────────────────┼──────────┼──────────────┼───────────────────────┼──────────────────────────┤");
    }

    // Realm sections
    for realm in &config.nodes.realms {
        let realm_backend = realm.backend.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No backend configuration found for realm {}", realm.id))?;
        let realm_storage = format_storage_info(realm_backend, Some(&format!("realm{}", realm.id)));

        let mut realm_services = Vec::new();
        let mut realm_total_cpu = 0;
        let mut realm_total_memory = 0;

        if realm.processor.enabled {
            if let Some(aws) = &realm.processor.aws {
                let task_count = match &aws.deployment_type {
                    Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => 1
                };
                realm_services.push(format!("Processor({})", task_count));
                realm_total_cpu += aws.cpu * task_count;
                realm_total_memory += aws.memory * task_count;
            }
        }

        if realm.edge.enabled {
            if let Some(aws) = &realm.edge.aws {
                let task_count = match &aws.deployment_type {
                    Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => 1
                };
                realm_services.push(format!("Edge({})", task_count));
                realm_total_cpu += aws.cpu * task_count;
                realm_total_memory += aws.memory * task_count;
            }
        }

        // if realm.worker.enabled {
        //     if let Some(aws) = &realm.worker.aws {
        //         let task_count = match &aws.deployment_type {
        //             Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
        //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
        //             _ => 1
        //         };
        //         realm_services.push(format!("Worker({})", task_count));
        //         realm_total_cpu += aws.cpu * task_count;
        //         realm_total_memory += aws.memory * task_count;
        //     }
        // }

        let realm_config_info = format!("Redis :{}, Edge :{}", 6380 + realm.id, 8546 + realm.id);

        // Build instance details for each service
        let mut realm_instance_details = Vec::new();
        if realm.processor.enabled {
            let rec = get_service_instance_recommendation(&realm.processor, "processor");
            realm_instance_details.push(format!("Processor: {}", rec));
        }
        if realm.edge.enabled {
            let rec = get_service_instance_recommendation(&realm.edge, "edge");
            realm_instance_details.push(format!("Edge: {}", rec));
        }
        // if realm.worker.enabled {
        //     let rec = get_service_instance_recommendation(&realm.worker, "worker");
        //     realm_instance_details.push(format!("Worker: {}", rec));
        // }

        if !realm_services.is_empty() {
            // Count total EC2 instances for this realm
            let mut realm_total_instances = 0;
            if realm.processor.enabled {
                if let Some(aws) = &realm.processor.aws {
                    realm_total_instances += match &aws.deployment_type {
                        Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                        _ => 1
                    };
                }
            }
            if realm.edge.enabled {
                if let Some(aws) = &realm.edge.aws {
                    realm_total_instances += match &aws.deployment_type {
                        Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                        _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                    };
                }
            }
            // if realm.worker.enabled {
            //     if let Some(aws) = &realm.worker.aws {
            //         realm_total_instances += match &aws.deployment_type {
            //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
            //             _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
            //         };
            //     }
            // }

            info!("│ {:<15} │ {:<32} │ {:<8} │ {:<12} │ {:<21} │                          │",
                format!("Realm {}", realm.id),
                realm_services.join("+"),
                format!("{:.1}", realm_total_cpu as f32 / 1024.0),
                format!("{:.0}GB", realm_total_memory as f32 / 1024.0),
                format!("{} total", realm_total_instances)
            );

            // Display instance details on separate lines
            for detail in realm_instance_details {
                info!("│                 │                                  │          │              │                       │ {:<24} │", detail);
            }

            info!("│                 │ {:<32} │          │              │                       │                          │",
                format!("Storage: {}", realm_storage)
            );
            info!("│                 │ {:<32} │          │              │                       │                          │",
                format!("Network: {}", realm_config_info)
            );
            if realm.id < config.nodes.realms.len() as u64 - 1 {
                info!("├─────────────────┼──────────────────────────────────┼──────────┼──────────────┼───────────────────────┼──────────────────────────┤");
            }
        }
    }

    // Prover section
    if let Some(prover) = &config.nodes.prover {
        if prover.enabled {
            if let Some(aws) = &prover.aws {
                let task_count = match &aws.deployment_type {
                    Some(DeploymentType::ECS) => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1),
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => 1
                };
                let prover_cpu = aws.cpu;
                let prover_memory = aws.memory;
                let prover_total_cpu = prover_cpu * task_count;
                let prover_total_memory = prover_memory * task_count;

                let prover_instances = match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                };

                info!("├─────────────────┼──────────────────────────────────┼──────────┼──────────────┼───────────────────────┼──────────────────────────┤");
                info!("│ {:<15} │ {:<32} │ {:<8} │ {:<12} │ {:<21} │                          │",
                    "Prover",
                    format!("Prover({})", task_count),
                    format!("{:.1}", prover_total_cpu as f32 / 1024.0),
                    format!("{:.0}GB", prover_total_memory as f32 / 1024.0),
                    format!("{} total", prover_instances)
                );
                info!("│                 │                                  │          │              │                       │ {:<24} │",
                    format!("Prover: {}", get_service_instance_recommendation(prover, "prover"))
                );
                // Add empty storage and network rows to match other components
                info!("│                 │ {:<32} │          │              │                       │                          │",
                    "Storage: None (stateless)"
                );
                info!("│                 │ {:<32} │          │              │                       │                          │",
                    "Network: Prover :8888"
                );
            }
        }
    }

    info!("└─────────────────┴──────────────────────────────────┴──────────┴──────────────┴───────────────────────┴──────────────────────────┘");

    // Calculate total EC2 instances and quota usage
    let mut total_ec2_vcpus = 0;

    // Calculate vCPUs for each service (assuming each task runs on its own EC2 instance)
    // Helper function to get instance vCPUs based on requirements
    let get_instance_vcpus = |cpu: u32| -> u32 {
        match cpu {
            0..=4096 => 4,   // xlarge (4 vCPU)
            4097..=8192 => 8, // 2xlarge (8 vCPU)
            _ => 16,          // 4xlarge (16 vCPU)
        }
    };

    // Coordinator services
    if config.nodes.coordinator.processor.enabled {
        if let Some(aws) = &config.nodes.coordinator.processor.aws {
            let instances = match &aws.deployment_type {
                Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
            };
            let vcpus = get_instance_vcpus(aws.cpu);
            total_ec2_vcpus += instances * vcpus;
        }
    }
    if config.nodes.coordinator.edge.enabled {
        if let Some(aws) = &config.nodes.coordinator.edge.aws {
            let instances = match &aws.deployment_type {
                Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
            };
            let vcpus = get_instance_vcpus(aws.cpu);
            total_ec2_vcpus += instances * vcpus;
        }
    }
    // if config.nodes.coordinator.worker.enabled {
    //     if let Some(aws) = &config.nodes.coordinator.worker.aws {
    //         let instances = match &aws.deployment_type {
    //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
    //             _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
    //         };
    //         let vcpus = get_instance_vcpus(aws.cpu);
    //         total_ec2_vcpus += instances * vcpus;
    //     }
    // }

    // Realm services
    for realm in &config.nodes.realms {
        if realm.processor.enabled {
            if let Some(aws) = &realm.processor.aws {
                let instances = match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                };
                let vcpus = get_instance_vcpus(aws.cpu);
                total_ec2_vcpus += instances * vcpus;
            }
        }
        if realm.edge.enabled {
            if let Some(aws) = &realm.edge.aws {
                let instances = match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                };
                let vcpus = get_instance_vcpus(aws.cpu);
                total_ec2_vcpus += instances * vcpus;
            }
        }
        // if realm.worker.enabled {
        //     if let Some(aws) = &realm.worker.aws {
        //         let instances = match &aws.deployment_type {
        //             Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
        //             _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
        //         };
        //         let vcpus = get_instance_vcpus(aws.cpu);
        //         total_ec2_vcpus += instances * vcpus;
        //     }
        // }
    }

    // Add this section for independent workers:
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            let mut total_worker_instances = 0;
            let mut total_worker_cpu = 0;
            let mut total_worker_memory = 0;

            for pool in &workers.worker_pools {
                total_worker_instances += pool.instances;
                if let Some(aws) = &pool.aws {
                    total_worker_cpu += aws.cpu * pool.instances;
                    total_worker_memory += aws.memory * pool.instances;
                }
            }

            if total_worker_instances > 0 {
                info!("├─────────────────┼──────────────────────────────────┼──────────┼──────────────┼───────────────────────┼──────────────────────────┤");
                info!("│ {:<15} │ {:<32} │ {:<8} │ {:<12} │ {:<21} │                          │",
                "Workers",
                format!("Independent({} pools)", workers.worker_pools.len()),
                format!("{:.1}", total_worker_cpu as f32 / 1024.0),
                format!("{:.0}GB", total_worker_memory as f32 / 1024.0),
                format!("{} total", total_worker_instances)
            );

                for (i, pool) in workers.worker_pools.iter().enumerate() {
                    let rec = if let Some(aws) = &pool.aws {
                        get_service_instance_recommendation(&ServiceConfig {
                            enabled: true,
                            instances: Some(pool.instances),
                            args: HashMap::new(),
                            env: HashMap::new(),
                            aws: Some(aws.clone()),
                        }, "worker")
                    } else {
                        "Local".to_string()
                    };
                    info!("│                 │                                  │          │              │                       │ Pool {}: {:<17} │", i+1, rec);
                }
            }
        }
    }
    // Prover service
    if let Some(prover) = &config.nodes.prover {
        if prover.enabled {
            if let Some(aws) = &prover.aws {
                let instances = match &aws.deployment_type {
                    Some(DeploymentType::EC2) => aws.ec2.as_ref().map(|e| e.desired_instances).unwrap_or(1),
                    _ => aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1)
                };
                let vcpus = get_instance_vcpus(aws.cpu);
                total_ec2_vcpus += instances * vcpus;
            }
        }
    }



    // Count storage instances only if any node is using ScyllaDB
    let has_scylla = config.nodes.coordinator.backend.as_ref()
        .map(|b| b.database == "scylla").unwrap_or(false) ||
        config.nodes.realms.iter().any(|r|
            r.backend.as_ref().map(|b| b.database == "scylla").unwrap_or(false));

    let storage_instances = if has_scylla {
        // Count ScyllaDB instances for coordinator and each realm
        let coordinator_scylla = config.nodes.coordinator.backend.as_ref()
            .and_then(|b| b.scylla.as_ref())
            .and_then(|s| s.aws.as_ref())
            .and_then(|aws| aws.ec2.as_ref())
            .map(|ec2| ec2.desired_instances)
            .unwrap_or(0);

        let realm_scylla: u32 = config.nodes.realms.iter()
            .map(|realm| {
                realm.backend.as_ref()
                    .and_then(|b| b.scylla.as_ref())
                    .and_then(|s| s.aws.as_ref())
            .and_then(|aws| aws.ec2.as_ref())
            .map(|ec2| ec2.desired_instances)
                    .unwrap_or(0)
            })
            .sum();

        coordinator_scylla + realm_scylla
    } else {
        0 // LMDBX doesn't need separate storage instances
    };

    // Calculate vCPUs for storage based on instance type
    // Get vCPUs from first ScyllaDB config (assume all use same instance type)
    let vcpus_per_instance = if let Some(scylla) = config.nodes.coordinator.backend.as_ref()
        .and_then(|b| b.scylla.as_ref()) {
        scylla.aws.as_ref()
            .map(|aws| aws.cpu / 1024) // Convert CPU units to vCPUs
            .unwrap_or(2)
    } else {
        2
    };
    let storage_vcpus = storage_instances * vcpus_per_instance;

    total_ec2_vcpus += storage_vcpus;

    let quota_usage = (total_ec2_vcpus as f32 / 128.0 * 100.0) as u32;

    info!("\n📊 Quota Summary:");
    info!("   Service instances: {} vCPUs", total_ec2_vcpus - storage_vcpus);
    if storage_instances > 0 {
        let num_clusters = if has_scylla {
            let mut clusters = 0;
            if config.nodes.coordinator.backend.as_ref()
                .map(|b| b.database == "scylla").unwrap_or(false) {
                clusters += 1;
            }
            clusters += config.nodes.realms.iter().filter(|r|
                r.backend.as_ref().map(|b| b.database == "scylla").unwrap_or(false)
            ).count();
            clusters
        } else {
            0
        };
        info!("   Storage (ScyllaDB): {} clusters × {} nodes × {} vCPUs = {} vCPUs",
            num_clusters, storage_instances / num_clusters as u32, vcpus_per_instance, storage_vcpus);
    } else {
        info!("   Storage: LMDBX (no separate instances needed)");
    }
    info!("   Total vCPUs: {} / 128 ({}% usage)", total_ec2_vcpus, quota_usage);

    if !recommendations.is_empty() {
        let total_hourly: f32 = recommendations.iter().map(|r| r.hourly_cost).sum();
        let total_monthly: f32 = recommendations.iter().map(|r| r.monthly_cost).sum();
        info!("   Estimated cost: ${:.2}/hour, ${:.0}/month", total_hourly, total_monthly);
    }


    Ok(())
}

    // AWS deployment file generation function
    async fn generate_aws_templates(config: &Config, args: GenerateAwsArgs) -> Result<()> {
        info!("Generating AWS deployment files...");

        // Create output directory
        fs::create_dir_all(&args.output_dir)
            .with_context(|| format!("Failed to create directory: {}", args.output_dir))?;

        // Create subdirectories
        let cf_dir = Path::new(&args.output_dir).join("cloudformation");
        fs::create_dir_all(&cf_dir)?;

        // Use simplified instance selector
        let selector = SimpleInstanceSelector::new();

        // Build service requirements from config
        let service_requirements = SimpleInstanceSelector::build_service_requirements_from_config(config);

        if service_requirements.is_empty() {
            warn!("No services requiring instance calculation found");
            return Ok(());
        }

        // Calculate instance recommendations (for ECS instance type selection)
        let recommendations = selector.calculate_multiple_recommendations(service_requirements)?;

        // Template file generation
        let templates = vec![
            ("cloudformation/main.yaml", include_str!("../../../.github/templates/aws/cloudformation/main.yaml.j2")),
            ("cloudformation/ecs-services.yaml", include_str!("../../../.github/templates/aws/cloudformation/ecs-services.yaml.j2")),
            ("deploy.sh", include_str!("../../../.github/templates/aws/deploy.sh.j2")),
        ];

        let mut env = Environment::new();

        // Generate template files
        for (filename, template_content) in templates {
            let output_path = Path::new(&args.output_dir).join(filename);
            if output_path.exists() && !args.force {
                warn!("File {} already exists, skipping (use --force to overwrite)", output_path.display());
                continue;
            }

            env.add_template(filename, template_content)?;
            let tmpl = env.get_template(filename)?;
            // Calculate required ECS instance type based on total task requirements
            let mut max_task_cpu = 0u32;
            let mut max_task_memory = 0u32;
            let mut total_task_cpu = 0u32;
            let mut total_task_memory = 0u32;
            let mut total_task_count = 0u32;

            // Check coordinator services
            if let Some(aws) = &config.nodes.coordinator.processor.aws {
                if let Some(DeploymentType::ECS) = &aws.deployment_type {
                    let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                    max_task_cpu = max_task_cpu.max(aws.cpu);
                    max_task_memory = max_task_memory.max(aws.memory);
                    total_task_cpu += aws.cpu * task_count;
                    total_task_memory += aws.memory * task_count;
                    total_task_count += task_count;
                }
            }
            if let Some(aws) = &config.nodes.coordinator.edge.aws {
                if let Some(DeploymentType::ECS) = &aws.deployment_type {
                    let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                    max_task_cpu = max_task_cpu.max(aws.cpu);
                    max_task_memory = max_task_memory.max(aws.memory);
                    total_task_cpu += aws.cpu * task_count;
                    total_task_memory += aws.memory * task_count;
                    total_task_count += task_count;
                }
            }
            // if let Some(aws) = &config.nodes.coordinator.worker.aws {
            //     if let Some(DeploymentType::ECS) = &aws.deployment_type {
            //         let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
            //         max_task_cpu = max_task_cpu.max(aws.cpu);
            //         max_task_memory = max_task_memory.max(aws.memory);
            //         total_task_cpu += aws.cpu * task_count;
            //         total_task_memory += aws.memory * task_count;
            //         total_task_count += task_count;
            //     }
            // }

            // Check realm services
            for realm in &config.nodes.realms {
                if let Some(aws) = &realm.processor.aws {
                    if let Some(DeploymentType::ECS) = &aws.deployment_type {
                        let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                        max_task_cpu = max_task_cpu.max(aws.cpu);
                        max_task_memory = max_task_memory.max(aws.memory);
                        total_task_cpu += aws.cpu * task_count;
                        total_task_memory += aws.memory * task_count;
                        total_task_count += task_count;
                    }
                }
                if let Some(aws) = &realm.edge.aws {
                    if let Some(DeploymentType::ECS) = &aws.deployment_type {
                        let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                        max_task_cpu = max_task_cpu.max(aws.cpu);
                        max_task_memory = max_task_memory.max(aws.memory);
                        total_task_cpu += aws.cpu * task_count;
                        total_task_memory += aws.memory * task_count;
                        total_task_count += task_count;
                    }
                }
                // if let Some(aws) = &realm.worker.aws {
                //     if let Some(DeploymentType::ECS) = &aws.deployment_type {
                //         let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                //         max_task_cpu = max_task_cpu.max(aws.cpu);
                //         max_task_memory = max_task_memory.max(aws.memory);
                //         total_task_cpu += aws.cpu * task_count;
                //         total_task_memory += aws.memory * task_count;
                //         total_task_count += task_count;
                //     }
                // }
            }

            // Check independent workers
            if let Some(workers) = &config.nodes.workers {
                if workers.enabled {
                    for pool in &workers.worker_pools {
                        if let Some(aws) = &pool.aws {
                            if let Some(DeploymentType::ECS) = &aws.deployment_type {
                                let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(pool.instances);
                                max_task_cpu = max_task_cpu.max(aws.cpu);
                                max_task_memory = max_task_memory.max(aws.memory);
                                total_task_cpu += aws.cpu * task_count;
                                total_task_memory += aws.memory * task_count;
                                total_task_count += task_count;
                            }
                        }
                    }
                }
            }

            // Check global API service
            if let Some(global_services) = &config.global_api_services {
                if let Some(api_service) = &global_services.api_service {
                    if api_service.enabled {
                        if let Some(aws) = &api_service.aws {
                            if let Some(DeploymentType::ECS) = &aws.deployment_type {
                                let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                                max_task_cpu = max_task_cpu.max(aws.cpu);
                                max_task_memory = max_task_memory.max(aws.memory);
                                total_task_cpu += aws.cpu * task_count;
                                total_task_memory += aws.memory * task_count;
                                total_task_count += task_count;
                            }
                        }
                    }
                }
            }

            // Check watcher services
            if let Some(watcher_config) = &config.nodes.coordinator.watcher {
                if watcher_config.enabled {
                    if let Some(aws) = &watcher_config.aws {
                        if let Some(DeploymentType::ECS) = &aws.deployment_type {
                            let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                            max_task_cpu = max_task_cpu.max(aws.cpu);
                            max_task_memory = max_task_memory.max(aws.memory);
                            total_task_cpu += aws.cpu * task_count;
                            total_task_memory += aws.memory * task_count;
                            total_task_count += task_count;
                        }
                    }
                }
            }
            for realm in &config.nodes.realms {
                if let Some(watcher_config) = &realm.watcher {
                    if watcher_config.enabled {
                        if let Some(aws) = &watcher_config.aws {
                            if let Some(DeploymentType::ECS) = &aws.deployment_type {
                                let task_count = aws.ecs.as_ref().map(|e| e.task_count).unwrap_or(1);
                                max_task_cpu = max_task_cpu.max(aws.cpu);
                                max_task_memory = max_task_memory.max(aws.memory);
                                total_task_cpu += aws.cpu * task_count;
                                total_task_memory += aws.memory * task_count;
                                total_task_count += task_count;
                            }
                        }
                    }
                }
            }

            // Calculate minimum instance type needed for ECS cluster
            // Note: This differs from the table display logic which shows optimal instance types
            // for each service individually. For ECS, we need ONE instance type that can handle
            // ANY task since tasks can be scheduled on any instance in the cluster.
            //
            // ECS container instances need to accommodate the largest single task
            let largest_task_cpu = max_task_cpu;
            let largest_task_memory = max_task_memory;

            debug!("ECS instance selection: largest_task_cpu={}, largest_task_memory={}",
                largest_task_cpu, largest_task_memory);

            // Select compute-optimized instance that can run the largest task
            let ecs_instance_type = match (largest_task_cpu, largest_task_memory) {
                (cpu, mem) if cpu <= 2048 && mem <= 4096 => "c6i.large",    // 2 vCPUs, 4GB
                (cpu, mem) if cpu <= 4096 && mem <= 8192 => "c6i.xlarge",   // 4 vCPUs, 8GB
                (cpu, mem) if cpu <= 8192 && mem <= 16384 => "c6i.2xlarge", // 8 vCPUs, 16GB
                (cpu, mem) if cpu <= 16384 && mem <= 32768 => "c6i.4xlarge", // 16 vCPUs, 32GB
                _ => "c6i.8xlarge", // 32 vCPUs, 64GB (fallback for larger tasks)
            };

            let output = tmpl.render(context! {
                config => config,
                recommendations => &recommendations,
                ecs_instance_type => ecs_instance_type,
            })?;

            fs::write(&output_path, output)
                .with_context(|| format!("Failed to write file {}", output_path.display()))?;

            // Make deploy.sh executable
            if filename == "deploy.sh" {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&output_path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&output_path, perms)?;
                }
            }

            info!("Generated {}", output_path.display());
        }

        info!("AWS deployment files generated to {}", args.output_dir);
        info!("Note: Use the Dockerfile in the project root to build the image");

        // Print deployment summary
        print_deployment_summary(config, &recommendations)?;


        Ok(())
    }
// Print deployment summary
fn print_deployment_summary2(config: &Config, database: &str) {
    info!("\n🚀 Deployment Summary");
    info!("=====================");
    info!("Backend: {}", database);

    // Coordinator
    info!("\n📍 Coordinator:");
    info!("  - Processor: {}", if config.nodes.coordinator.processor.enabled { "✅" } else { "❌" });
    info!("  - Edge: {}", if config.nodes.coordinator.edge.enabled { "✅" } else { "❌" });
    if let Some(watcher) = &config.nodes.coordinator.watcher {
        info!("  - Watcher: {}", if watcher.enabled { "✅" } else { "❌" });
    }

    // Realms
    info!("\n🌍 Realms:");
    for realm in &config.nodes.realms {
        info!("  Realm {}:", realm.id);
        info!("    - Processor: {}", if realm.processor.enabled { "✅" } else { "❌" });
        info!("    - Edge: {}", if realm.edge.enabled { "✅" } else { "❌" });
        if let Some(watcher) = &realm.watcher {
            info!("    - Watcher: {}", if watcher.enabled { "✅" } else { "❌" });
        }
    }

    // Workers
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            info!("\n👷 Workers:");
            for pool in &workers.worker_pools {
                info!("  - Pool '{}': {} instances", pool.id, pool.instances);
            }
        }
    }

    // Global Services
    if let Some(global_services) = &config.global_api_services {
        info!("\n🌐 Global Services:");
        if let Some(api) = &global_services.api_service {
            info!("  - API Service: {} (port 3000)", if api.enabled { "✅" } else { "❌" });
        }
        if let Some(timescale) = &global_services.timescaledb {
            info!("  - TimescaleDB: {}", if timescale.enabled { "✅" } else { "❌" });
        }
    }

    info!("\n📡 Endpoints:");
    info!("  - Coordinator: http://localhost:8545");
    for realm in &config.nodes.realms {
        info!("  - Realm {}: http://localhost:{}", realm.id, 8546 + realm.id);
    }
    if let Some(global_services) = &config.global_api_services {
        if let Some(api) = &global_services.api_service {
            if api.enabled {
                info!("  - API Service: http://localhost:3000");
            }
        }
    }
}


/// Get instance recommendation for a specific service
fn get_service_instance_recommendation(service_config: &ServiceConfig, service_type: &str) -> String {
    if let Some(aws_config) = &service_config.aws {
        let task_count = match &aws_config.deployment_type {
            Some(DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
            Some(DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
            _ => 1
        };

        let vcpus = aws_config.cpu / 1024;
        let memory_gb = aws_config.memory / 1024;

        // Choose instance type based on service type and requirements
        let instance_type = match service_type {
            "processor" => {
                if vcpus <= 4 && memory_gb <= 8 {
                    "c6i.xlarge"
                } else if vcpus <= 8 && memory_gb <= 16 {
                    "c6i.2xlarge"
                } else {
                    "c6i.4xlarge"
                }
            },
            "edge" => {
                if vcpus <= 4 && memory_gb <= 16 {
                    "m6i.xlarge"
                } else if vcpus <= 8 && memory_gb <= 32 {
                    "m6i.2xlarge"
                } else {
                    "m6i.4xlarge"
                }
            },
            "worker" | "prover" => {
                if vcpus <= 4 && memory_gb <= 8 {
                    "c6i.xlarge"
                } else if vcpus <= 8 && memory_gb <= 16 {
                    "c6i.2xlarge"
                } else {
                    "c6i.4xlarge"
                }
            },
            _ => "m6i.xlarge"
        };

        format!("{}({})", instance_type, task_count)
    } else {
        "-".to_string()
    }
}

/// Format deployment type and cost information
fn format_deployment_info(service_config: &ServiceConfig, recommendation_map: &std::collections::HashMap<String, &SimpleInstanceRecommendation>, group_name: &str) -> String {
    if let Some(aws_config) = &service_config.aws {
        let deployment_type = aws_config.deployment_type.as_ref()
            .map(|dt| match dt {
                DeploymentType::ECS => "ECS",
                DeploymentType::EC2 => "EC2",
            })
            .unwrap_or("ECS"); // Default to ECS

        let instance_info = if let Some(rec) = recommendation_map.get(group_name) {
            format!("{} x{} (${:.1}/h)", rec.instance_type.name, rec.instance_count, rec.hourly_cost)
        } else {
            // Calculate on-the-fly if no recommendation exists
            let vcpus = aws_config.cpu / 1024; // Convert from CPU units to vCPUs
            let memory_gb = aws_config.memory as f32 / 1024.0; // Convert from MB to GB
            // Use simple instance selector to get recommendation
            let selector = SimpleInstanceSelector::new();
            let task_count = match &aws_config.deployment_type {
                Some(DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                Some(DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
                _ => 1
            };
            let requirements = crate::aws::simple_instance_selector::ServiceGroupRequirements {
                name: group_name.to_string(),
                service_type: crate::aws::simple_instance_selector::ServiceType::Worker, // Default type
                total_vcpus: vcpus,
                total_memory_gb: memory_gb,
                instance_count: task_count,
            };

            match selector.calculate_recommendation(&requirements) {
                Ok(rec) => format!("{} x{} (${:.1}/h)", rec.instance_type.name, rec.instance_count, rec.hourly_cost),
                Err(_) => format!("{} {}vCPU/{}GB", deployment_type, vcpus, memory_gb as u32)
            }
        };

        instance_info
    } else {
        "Local".to_string()
    }
}

/// Format storage information
fn format_storage_info(backend: &BackendConfig, suffix: Option<&str>) -> String {
    match backend.database.as_str() {
        "scylla" => {
            let cluster_size = backend.scylla.as_ref()
                .and_then(|s| s.aws.as_ref())
            .and_then(|aws| aws.ec2.as_ref())
            .map(|ec2| ec2.desired_instances)
                .unwrap_or(1);
            format!("ScyllaDB ({} nodes)", cluster_size)
        },
        "lmdbx" => {
            if let Some(lmdbx) = &backend.lmdbx {
                format!("LMDBX ({}GB)", lmdbx.mmap_size_gb)
            } else {
                "LMDBX".to_string()
            }
        },
        _ => backend.database.clone(),
    }
}

/// Print storage details
fn print_storage_details(config: &Config) -> Result<()> {
    info!("\n💾 Storage Details:");

    // Redis details
    info!("\nRedis Instances:");
    if let Some(coordinator_redis) = &config.nodes.coordinator.redis {
        info!("  - Coordinator: {}", coordinator_redis.uri);
    }
    for realm in &config.nodes.realms {
        if let Some(realm_redis) = &realm.redis {
            info!("  - Realm {}: {}", realm.id, realm_redis.uri);
        }
    }

    // ScyllaDB details
    let mut scylla_clusters: Vec<(String, u32, String)> = Vec::new();

    // Check coordinator
    let coord_backend = config.nodes.coordinator.backend.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No backend configuration found for coordinator"))?;
    if coord_backend.database == "scylla" {
        if let Some(scylla) = &coord_backend.scylla {
            let cluster_size = scylla.aws.as_ref()
                .and_then(|aws| aws.ec2.as_ref())
                .map(|ec2| ec2.desired_instances)
                .unwrap_or(3);
            scylla_clusters.push(("Coordinator".to_string(), cluster_size, "scylla-coordinator".to_string()));
        }
    }

    // Check realms
    for realm in &config.nodes.realms {
        let realm_backend = realm.backend.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No backend configuration found for realm {}", realm.id))?;
        if realm_backend.database == "scylla" {
            if let Some(scylla) = &realm_backend.scylla {
                let cluster_size = scylla.aws.as_ref()
                .and_then(|aws| aws.ec2.as_ref())
                .map(|ec2| ec2.desired_instances)
                .unwrap_or(3);
                scylla_clusters.push((
                    format!("Realm {}", realm.id),
                    cluster_size,
                    format!("scylla-realm{}", realm.id)
                ));
            }
        }
    }

    if !scylla_clusters.is_empty() {
        info!("\nScyllaDB Clusters:");
        for (name, size, prefix) in scylla_clusters {
            info!("  - {} ScyllaDB: {} nodes", name, size);
            for i in 1..=size {
                info!("    • {}-{}:9042", prefix, i);
            }
        }
    }

    Ok(())
}

/// Print simplified Docker Compose summary without AWS details
fn print_docker_compose_summary(config: &Config) -> Result<()> {
    info!("\n🐳 Docker Compose Summary");
    info!("═══════════════════════════════════════════════════════════════");

    // Count containers
    let mut total_containers = 0u32;

    // Redis containers
    if config.nodes.coordinator.redis.is_some() {
        total_containers += 1;
    }
    for realm in &config.nodes.realms {
        if realm.redis.is_some() {
            total_containers += 1;
        }
    }

    // Service containers
    if config.nodes.coordinator.processor.enabled {
        total_containers += 1;
    }
    if config.nodes.coordinator.edge.enabled {
        total_containers += 1;
    }

    // Add coordinator watcher
    if let Some(watcher) = &config.nodes.coordinator.watcher {
        if watcher.enabled {
            total_containers += 1;
        }
    }

    for realm in &config.nodes.realms {
        if realm.processor.enabled {
            total_containers += 1;
        }
        if realm.edge.enabled {
            total_containers += 1;
        }

        // Add realm watcher
        if let Some(watcher) = &realm.watcher {
            if watcher.enabled {
                total_containers += 1;
            }
        }
    }

    // Add independent workers
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            for pool in &workers.worker_pools {
                total_containers += pool.instances;
            }
        }
    }

    // Add global API service
    if let Some(global_services) = &config.global_api_services {
        if let Some(api_service) = &global_services.api_service {
            if api_service.enabled {
                total_containers += 1;
            }
        }

        // Add TimescaleDB container (if using Docker deployment)
        if let Some(timescale_config) = &global_services.timescaledb {
            if timescale_config.enabled {
                total_containers += 1;
            }
        }
    }

    if let Some(prover) = &config.nodes.prover {
        if prover.enabled {
            total_containers += 1;
        }
    }

    info!("📊 Total Containers: {}", total_containers);

    // Network configuration
    let network = &config.nodes.deployment.docker;
    info!("🌐 Network: {} ({})", network.network_name, network.network_subnet);

    // Service endpoints
    info!("\n🔗 Service Endpoints:");

    // Coordinator endpoint
    if config.nodes.coordinator.edge.enabled {
        if let Some(listen_addr) = config.nodes.coordinator.edge.args.get("listen_addr") {
            if let Value::String(addr) = listen_addr {
                let port = addr.split(':').last().unwrap_or("8545");
                info!("  • Coordinator: http://localhost:{}", port);
            } else {
                info!("  • Coordinator: http://localhost:8545");
            }
        } else {
            info!("  • Coordinator: http://localhost:8545");
        }
    }

    // Realm endpoints
    for realm in &config.nodes.realms {
        if realm.edge.enabled {
            if let Some(listen_addr) = realm.edge.args.get("listen_addr") {
                if let Value::String(addr) = listen_addr {
                    let port = addr.split(':').last().unwrap_or("8546");
                    info!("  • Realm {}: http://localhost:{}", realm.id, port);
                }
            }
        }
    }

    // API Service endpoint
    if let Some(global_services) = &config.global_api_services {
        if let Some(api_service) = &global_services.api_service {
            if api_service.enabled {
                if let Some(listen_addr) = api_service.args.get("listen_addr") {
                    if let Value::String(addr) = listen_addr {
                        let port = addr.split(':').last().unwrap_or("3000");
                        info!("  • API Service: http://localhost:{}", port);
                    } else {
                        info!("  • API Service: http://localhost:3000");
                    }
                } else {
                    info!("  • API Service: http://localhost:3000");
                }
            }
        }
    }

    // Prover endpoint
    if let Some(prover) = &config.nodes.prover {
        if prover.enabled {
            if let Some(listen_addr) = prover.args.get("listen_addr") {
                if let Value::String(addr) = listen_addr {
                    let port = addr.split(':').last().unwrap_or("8888");
                    info!("  • Prover: http://localhost:{}", port);
                } else {
                    info!("  • Prover: http://localhost:8888");
                }
            } else {
                info!("  • Prover: http://localhost:8888");
            }
        }
    }

    // Worker pools summary
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            info!("\n👷 Worker Pools:");
            for pool in &workers.worker_pools {
                let total_instances = pool.instances;

                info!("  • Pool '{}': {} instances",
                    pool.id,
                    total_instances
                );

                // Show worker configuration details
                if let Some(aws) = &pool.aws {
                    info!("    Resources: {} CPU units, {} MB memory",
                        aws.cpu,
                        aws.memory
                    );
                }

                // Workers will auto-discover nodes from config
                info!("    Node discovery: via config.json RPC endpoints");
                info!("    API reporting: port 3000");
            }

            // Show total worker count
            let total_workers: u32 = workers.worker_pools.iter()
                .map(|p| p.instances)
                .sum();
            info!("  Total workers: {}", total_workers);
        }
    }

    info!("\n💡 Quick Start:");
    info!("   docker-compose up -d");
    info!("   docker-compose logs -f");

    Ok(())
}


fn default_data_volume_size() -> u32 {
    500
}

fn default_commitlog_volume_size() -> u32 {
    100
}
fn default_pd_count() -> u32 {
    3
}

fn default_tikv_count() -> u32 {
    3
}