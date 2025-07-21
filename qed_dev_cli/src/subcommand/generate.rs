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
use tracing::{info, warn, error};
use tokio::time::{sleep, Duration};

use super::{GenerateArgs, GenerateCommands, RunArgs, GenerateDockerComposeArgs, GenerateAwsArgs};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub nodes: NodesConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub users_per_realm: u64,
    pub global_user_tree_height: u8,
    pub realm_user_tree_height: u8,
    pub realm_configs: Vec<RealmConfig>,
    pub coordinator_configs: Vec<CoordinatorConfig>,
    pub prover_url: String,
    pub native_currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RealmConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub id: u64,
    pub rpc_url: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodesConfig {
    pub backend: BackendConfig,
    pub redis: RedisConfig,
    pub coordinator: NodeGroup,
    pub realms: Vec<RealmNode>,
    pub prover: Option<ServiceConfig>,
    pub deployment: DeploymentConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackendConfig {
    #[serde(rename = "type")]
    pub database: String,
    pub lmdbx: Option<LmdbxConfig>,
    pub scylla: Option<ScyllaConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LmdbxConfig {
    pub base_path: String,
    pub size_gb: u32,
    pub volume_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScyllaConfig {
    pub endpoints: Vec<String>,
    pub replication_factor: u32,
    pub consistency_level: String,
    pub cluster_size: Option<u32>,
    pub instance_type: Option<String>,
    pub data_volume_size: Option<u32>,
    pub commitlog_volume_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisConfig {
    pub coordinator: RedisInstance,
    pub realms: Vec<RedisRealm>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisInstance {
    pub uri: String,
    pub pool_size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisRealm {
    pub id: u64,
    pub uri: String,
    pub pool_size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeGroup {
    pub id: u64,
    pub backend: Option<BackendConfig>,  // Optional, falls back to global backend
    pub processor: ServiceConfig,
    pub edge: ServiceConfig,
    pub worker: ServiceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RealmNode {
    pub id: u64,
    pub node_id: u64,
    pub backend: Option<BackendConfig>,  // Optional, falls back to global backend
    pub processor: ServiceConfig,
    pub edge: ServiceConfig,
    pub worker: ServiceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub instances: Option<u32>,
    pub args: HashMap<String, Value>,
    pub env: HashMap<String, String>,
    pub aws: Option<AwsServiceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsServiceConfig {
    pub cpu: u32,
    pub memory: u32,
    pub task_count: u32,
    pub load_balancer: Option<LoadBalancerConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub port: u16,
    pub health_check_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub aws: AwsDeploymentConfig,
    pub docker: DockerDeploymentConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwsDeploymentConfig {
    pub region: String,
    pub project_name: String,
    pub vpc: VpcConfig,
    pub ecs: EcsConfig,
    pub ecr: EcrConfig,
    pub s3: S3Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VpcConfig {
    pub cidr: String,
    pub availability_zones: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EcsConfig {
    pub cluster_name: String,
    pub log_group: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EcrConfig {
    pub repository_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket_prefix: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
        .unwrap_or(&config.nodes.backend.database);

    // Create necessary directories
    if database == "lmdbx" {
        if let Some(lmdbx_config) = &config.nodes.backend.lmdbx {
            fs::create_dir_all(&lmdbx_config.base_path)
                .with_context(|| format!("Failed to create directory: {}", lmdbx_config.base_path))?;
        }
    }

    // Start Redis instances
    start_redis_instances(config)?;

    // Start ScyllaDB if using scylla backend
    if database == "scylla" {
        start_scylladb(config)?;
    }

    // Start coordinator services
    start_coordinator_services(config, database, &args)?;

    // Start realm services
    start_realm_services(config, database, &args)?;

    info!("QED network deployment started successfully!");

    if !args.detach {
        info!("Press Ctrl+C to stop the deployment...");
        tokio::signal::ctrl_c().await?;
        stop_deployment().await?;
    }

    Ok(())
}

fn start_redis_instances(config: &Config) -> Result<()> {
    info!("Starting Redis instances...");

    // Extract port from coordinator URI
    let coordinator_port = extract_port(&config.nodes.redis.coordinator.uri)?;
    start_redis_instance(coordinator_port, "coordinator")?;

    // Start realm Redis instances
    for realm_redis in &config.nodes.redis.realms {
        let port = extract_port(&realm_redis.uri)?;
        start_redis_instance(port, &format!("realm_{}", realm_redis.id))?;
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

    let redis_uri = &config.nodes.redis.coordinator.uri;
    let node = &config.nodes.coordinator;

    // Use node-specific backend or fall back to global
    let backend_config = node.backend.as_ref().unwrap_or(&config.nodes.backend);
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
                args.log_level.as_deref(),
                Some(backend_config),
            )?,
            &node.processor.env,
        )?;
    }

    // Start workers
    if node.worker.enabled {
        let instances = node.worker.instances.unwrap_or(1);
        for i in 0..instances {
            start_service(
                &format!("coordinator-worker-{}", i),
                build_service_command(
                    "coordinator-worker",
                    node_database,
                    redis_uri,
                    None,
                    &node.worker,
                    config,
                    args.log_level.as_deref(),
                    Some(backend_config),
                )?,
                &node.worker.env,
            )?;
        }
    }

    // Wait a bit before starting edge
    std::thread::sleep(std::time::Duration::from_secs(2));

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
                args.log_level.as_deref(),
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
        let realm_redis = config.nodes.redis.realms.iter()
            .find(|r| r.id == realm_node.id)
            .ok_or_else(|| anyhow::anyhow!("Redis config not found for realm {}", realm_node.id))?;

        let redis_uri = &realm_redis.uri;

        // Use realm-specific backend or fall back to global
        let backend_config = realm_node.backend.as_ref().unwrap_or(&config.nodes.backend);
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
                    args.log_level.as_deref(),
                    Some(backend_config),
                )?,
                &realm_node.processor.env,
            )?;
        }

        // Start workers
        if realm_node.worker.enabled {
            let instances = realm_node.worker.instances.unwrap_or(1);
            for i in 0..instances {
                start_service(
                    &format!("realm-{}-worker-{}", realm_node.id, i),
                    build_service_command(
                        "realm-worker",
                        realm_database,
                        redis_uri,
                        Some(realm_node),
                        &realm_node.worker,
                        config,
                        args.log_level.as_deref(),
                        Some(backend_config),
                    )?,
                    &realm_node.worker.env,
                )?;
            }
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
                    args.log_level.as_deref(),
                    Some(backend_config),
                )?,
                &realm_node.edge.env,
            )?;
        }
    }

    Ok(())
}

fn build_service_command(
    service_type: &str,
    database: &str,
    redis_uri: &str,
    realm_node: Option<&RealmNode>,
    service_config: &ServiceConfig,
    config: &Config,
    override_log_level: Option<&str>,
    backend_config: Option<&BackendConfig>,
) -> Result<Vec<String>> {
    let mut cmd = vec![
        "./target/release/qed_rollup_cli".to_string(),
        service_type.to_string(),
    ];

    // Add backend-specific args
    let backend = backend_config.unwrap_or(&config.nodes.backend);

    match database {
        "lmdbx" => {
            cmd.push("--database".to_string());
            cmd.push("lmdbx".to_string());

            if let Some(lmdbx_config) = &backend.lmdbx {
                let path = if let Some(realm) = realm_node {
                    format!("{}/realm{}", lmdbx_config.base_path, realm.id)
                } else {
                    format!("{}/coordinator", lmdbx_config.base_path)
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
        _ => return Err(anyhow::anyhow!("Unknown backend type: {}", database)),
    }

    // Add Redis URI
    cmd.push("--redis-uri".to_string());
    cmd.push(redis_uri.to_string());

    // Add realm-specific args
    if let Some(realm) = realm_node {
        cmd.push("--node-id".to_string());
        cmd.push(realm.node_id.to_string());
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

    // Override log level if specified
    if let Some(log_level) = override_log_level {
        cmd.push("--log-level".to_string());
        cmd.push(log_level.to_string());
    }

    Ok(cmd)
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

    let tmpl = env.get_template("docker-compose")?;
    let output = tmpl.render(context! {
        config => config,
    })?;

    fs::write(&args.output, output)
        .with_context(|| format!("Failed to write docker-compose.yml to {}", args.output))?;

    info!("Generated docker-compose.yml at {}", args.output);
    Ok(())
}

async fn generate_aws_templates(config: &Config, args: GenerateAwsArgs) -> Result<()> {
    info!("Generating AWS deployment files...");

    // Create output directory
    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("Failed to create directory: {}", args.output_dir))?;

    // Create subdirectories
    let cf_dir = Path::new(&args.output_dir).join("cloudformation");
    fs::create_dir_all(&cf_dir)?;

    // Template files to generate
    let templates = vec![
        ("cloudformation/main.yaml", include_str!("../../../.github/templates/aws/cloudformation/main.yaml.j2")),
        ("cloudformation/ecs-services.yaml", include_str!("../../../.github/templates/aws/cloudformation/ecs-services.yaml.j2")),
        ("deploy.sh", include_str!("../../../.github/templates/aws/deploy.sh.j2")),
    ];

    let mut env = Environment::new();

    // Generate templates
    for (filename, template_content) in templates {
        let output_path = Path::new(&args.output_dir).join(filename);
        if output_path.exists() && !args.force {
            warn!("File {} already exists, skipping (use --force to overwrite)", output_path.display());
            continue;
        }

        env.add_template(filename, template_content)?;
        let tmpl = env.get_template(filename)?;
        let output = tmpl.render(context! {
            config => config,
        })?;

        fs::write(&output_path, output)
            .with_context(|| format!("Failed to write {}", output_path.display()))?;

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

    info!("AWS deployment files generated in {}", args.output_dir);
    info!("Note: Use the Dockerfile from the project root directory for building images");

    // Calculate and display AWS cost estimation
    estimate_aws_costs(config);

    Ok(())
}

fn estimate_aws_costs(config: &Config) {
    info!("\n=== AWS Deployment Cost Estimation ===");

    let mut monthly_cost = 0.0;
    let mut daily_cost = 0.0;

    // EC2 Instances - based on actual usage patterns
    // c6i.2xlarge: $0.34/hour (8 vCPU, 16GB RAM) - for compute workers
    // r6i.2xlarge: $0.504/hour (8 vCPU, 64GB RAM) - for memory-intensive services
    // Typically 3-5 instances total
    let compute_instances = 3;  // c6i.2xlarge for workers
    let memory_instances = 2;   // r6i.2xlarge for processors/edge
    let c6i_2xlarge_rate = 0.34;
    let r6i_2xlarge_rate = 0.504;
    
    let ec2_compute_daily = compute_instances as f64 * c6i_2xlarge_rate * 24.0;
    let ec2_memory_daily = memory_instances as f64 * r6i_2xlarge_rate * 24.0;
    let ec2_daily = ec2_compute_daily + ec2_memory_daily;
    daily_cost += ec2_daily;
    
    info!("EC2 Compute ({}x c6i.2xlarge @ $0.34/hr): ${:.2}/day", compute_instances, ec2_compute_daily);
    info!("EC2 Memory ({}x r6i.2xlarge @ $0.504/hr): ${:.2}/day", memory_instances, ec2_memory_daily);
    info!("EC2 Instances Total: ${:.2}/day", ec2_daily);

    // EBS Storage (gp3) - with IOPS and throughput costs
    let total_instances = compute_instances + memory_instances;
    let ebs_gb = 200 * total_instances; // 200GB per instance
    let ebs_rate_per_gb_month = 0.08;
    let ebs_iops_rate = 0.005; // per IOPS-month for gp3
    let ebs_throughput_rate = 0.04; // per MB/s-month
    
    let ebs_storage_daily = (ebs_gb as f64 * ebs_rate_per_gb_month) / 30.0;
    let ebs_iops_daily = (3000.0 * total_instances as f64 * ebs_iops_rate) / 30.0; // 3000 IOPS per volume
    let ebs_throughput_daily = (125.0 * total_instances as f64 * ebs_throughput_rate) / 30.0; // 125 MB/s per volume
    let ebs_daily = ebs_storage_daily + ebs_iops_daily + ebs_throughput_daily;
    
    daily_cost += ebs_daily;
    info!("EBS Storage ({}GB gp3): ${:.2}/day", ebs_gb, ebs_storage_daily);
    info!("EBS IOPS & Throughput: ${:.2}/day", ebs_iops_daily + ebs_throughput_daily);
    info!("EBS Total: ${:.2}/day", ebs_daily);

    // ElastiCache Redis (cache.t3.micro - $0.017/hour)
    let redis_count = 3; // coordinator + 2 realms
    let redis_hourly_rate = 0.017;
    let redis_daily = redis_count as f64 * redis_hourly_rate * 24.0;
    daily_cost += redis_daily;
    info!("ElastiCache Redis ({}x cache.t3.micro): ${:.2}/day", redis_count, redis_daily);

    // Application Load Balancer
    let alb_hourly_rate = 0.0225;
    let alb_lcu_rate = 0.008; // per LCU hour
    let alb_daily = alb_hourly_rate * 24.0 + alb_lcu_rate * 24.0 * 10.0; // Higher LCU usage for real workloads
    daily_cost += alb_daily;
    info!("Application Load Balancer: ${:.2}/day", alb_daily);

    // NAT Gateway (MAJOR COST - often overlooked)
    let nat_gateway_count = 2; // One per AZ
    let nat_gateway_hourly = 0.045;
    let nat_gateway_data_rate = 0.045; // per GB processed
    let nat_gateway_gb_daily = 300.0; // Container images, updates, etc.
    
    let nat_gateway_fixed_daily = nat_gateway_count as f64 * nat_gateway_hourly * 24.0;
    let nat_gateway_data_daily = nat_gateway_gb_daily * nat_gateway_data_rate;
    let nat_gateway_daily = nat_gateway_fixed_daily + nat_gateway_data_daily;
    daily_cost += nat_gateway_daily;
    info!("NAT Gateway Fixed ({}x): ${:.2}/day", nat_gateway_count, nat_gateway_fixed_daily);
    info!("NAT Gateway Data ({}GB/day): ${:.2}/day", nat_gateway_gb_daily, nat_gateway_data_daily);

    // Data Transfer (realistic estimates)
    let internet_transfer_gb_month = 1000.0; // 1TB/month internet egress
    let cross_az_gb_month = 2000.0; // 2TB/month cross-AZ
    let data_transfer_rate = 0.09; // Internet egress
    let cross_az_rate = 0.01; // per GB each direction
    
    let internet_daily = (internet_transfer_gb_month * data_transfer_rate) / 30.0;
    let cross_az_daily = (cross_az_gb_month * cross_az_rate * 2.0) / 30.0;
    let data_transfer_daily = internet_daily + cross_az_daily;
    daily_cost += data_transfer_daily;
    info!("Internet Egress ({}GB/mo): ${:.2}/day", internet_transfer_gb_month, internet_daily);
    info!("Cross-AZ Transfer ({}GB/mo): ${:.2}/day", cross_az_gb_month, cross_az_daily);

    // CloudWatch (comprehensive)
    let cloudwatch_logs_ingestion = 50.0; // GB/month
    let cloudwatch_logs_storage = 100.0; // GB stored
    let cloudwatch_metrics = 50; // custom metrics
    
    let logs_ingestion_daily = (cloudwatch_logs_ingestion * 0.50) / 30.0;
    let logs_storage_daily = (cloudwatch_logs_storage * 0.03) / 30.0;
    let metrics_daily = (cloudwatch_metrics as f64 * 0.30) / 30.0;
    let cloudwatch_daily = logs_ingestion_daily + logs_storage_daily + metrics_daily + 5.0; // +$5 for dashboards, alarms
    daily_cost += cloudwatch_daily;
    info!("CloudWatch Total: ${:.2}/day", cloudwatch_daily);

    // ECR Storage
    let ecr_storage_gb = 30.0;
    let ecr_daily = (ecr_storage_gb * 0.10) / 30.0;
    daily_cost += ecr_daily;
    info!("ECR Storage ({}GB): ${:.2}/day", ecr_storage_gb, ecr_daily);

    // ScyllaDB if enabled
    if config.nodes.backend.database == "scylla" {
        // r6i.2xlarge - $0.504/hour, 3 instances (matching actual usage)
        let scylla_count = 3;
        let scylla_hourly_rate = 0.504;
        let scylla_daily = scylla_count as f64 * scylla_hourly_rate * 24.0;
        daily_cost += scylla_daily;

        // ScyllaDB EBS storage (500GB data + 100GB commitlog per instance)
        let scylla_ebs_gb = 600 * scylla_count;
        let scylla_ebs_storage_daily = (scylla_ebs_gb as f64 * ebs_rate_per_gb_month) / 30.0;
        let scylla_ebs_iops_daily = (16000.0 * scylla_count as f64 * 2.0 * ebs_iops_rate) / 30.0; // 16k IOPS for data + commitlog
        let scylla_ebs_throughput_daily = (1000.0 * scylla_count as f64 * 2.0 * ebs_throughput_rate) / 30.0; // 1000 MB/s
        let scylla_ebs_daily = scylla_ebs_storage_daily + scylla_ebs_iops_daily + scylla_ebs_throughput_daily;
        daily_cost += scylla_ebs_daily;

        info!("ScyllaDB Instances ({}x r6i.2xlarge @ $0.504/hr): ${:.2}/day", scylla_count, scylla_daily);
        info!("ScyllaDB Storage ({}GB): ${:.2}/day", scylla_ebs_gb, scylla_ebs_storage_daily);
        info!("ScyllaDB IOPS & Throughput: ${:.2}/day", scylla_ebs_iops_daily + scylla_ebs_throughput_daily);
    }

    monthly_cost = daily_cost * 30.0;

    info!("\n--- Total Estimated Costs ---");
    info!("Daily: ${:.2}", daily_cost);
    info!("Monthly (30 days): ${:.2}", monthly_cost);
    info!("Yearly: ${:.2}", monthly_cost * 12.0);

    info!("\n💡 Cost Optimization Tips:");
    info!("- Use Reserved Instances for 1-3 year commitments (save up to 72%)");
    info!("- Use Spot Instances for workers (save up to 90%)");
    info!("- Use smaller instance types for development/testing");
    info!("- Enable auto-scaling to reduce costs during low usage");
    info!("- Consider using AWS Graviton instances (up to 40% better price/performance)");

    if monthly_cost > 500.0 {
        warn!("\n⚠️  Monthly cost exceeds $500. Consider optimization strategies.");
    }
}
