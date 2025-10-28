use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader};
use std::thread;
use tracing::{info, warn, error, debug};
use tokio::time::{sleep, Duration};

/// Process manager to track all spawned processes
struct ProcessManager {
    processes: Arc<Mutex<Vec<ProcessInfo>>>,
}

struct ProcessInfo {
    name: String,
    child: Child,
    log_file: PathBuf,
}

impl ProcessManager {
    fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn spawn(&self, name: String, mut cmd: Command, log_dir: &Path) -> Result<()> {
        // Create log files
        let log_file = log_dir.join(format!("{}.log", name));
        let err_file = log_dir.join(format!("{}.err", name));

        let out_file = fs::File::create(&log_file)?;
        let err_file = fs::File::create(&err_file)?;

        info!("🚀 Starting {}", name);
        debug!("Command: {:?}", cmd);

        // Spawn process
        let child = cmd
            .stdout(Stdio::from(out_file))
            .stderr(Stdio::from(err_file))
            .spawn()
            .with_context(|| format!("Failed to spawn {}", name))?;

        // Track process
        let mut processes = self.processes.lock().unwrap();
        processes.push(ProcessInfo {
            name: name.clone(),
            child,
            log_file: log_file.clone(),
        });

        Ok(())
    }

    fn kill_all(&self) {
        let mut processes = self.processes.lock().unwrap();
        for mut proc in processes.drain(..) {
            info!("🛑 Stopping {}", proc.name);
            let _ = proc.child.kill();
        }
    }

    fn wait_all(&self) -> Result<()> {
        let mut processes = self.processes.lock().unwrap();
        for proc in processes.iter_mut() {
            let status = proc.child.wait()?;
            if !status.success() {
                error!("{} exited with status: {}", proc.name, status);
            }
        }
        Ok(())
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

// Load config.json structure
use super::generate::{Config, NodesConfig, BackendConfig, RedisConfig, CoordinatorNode, RealmNode, ServiceConfig};
use super::LaunchArgs;

/// Entry point for the launch command
pub async fn run(args: LaunchArgs) -> Result<()> {
    launch(args.config, args.verbose).await
}

/// Main launch function inspired by polkadot-launch
pub async fn launch(config_path: Option<String>, verbose: bool) -> Result<()> {
    // Load configuration from config.json
    let config_file = config_path.unwrap_or_else(|| "config.json".to_string());
    let content = fs::read_to_string(&config_file)
        .with_context(|| format!("Failed to read config file: {}", config_file))?;
    let config: Config = serde_json::from_str(&content)
        .with_context(|| "Failed to parse config.json")?;

    if verbose {
        debug!("Configuration: {:#?}", config);
    }

    // Create working directory
    let work_dir = PathBuf::from(".");
    let log_dir = work_dir.join("logs");
    fs::create_dir_all(&log_dir)?;

    info!("🚀 QED Launch - Starting development environment");
    info!("📁 Working directory: {}", work_dir.display());
    info!("📝 Log directory: {}", log_dir.display());

    let manager = ProcessManager::new();

    // Phase 1: Start infrastructure
    info!("🏗️  Phase 1: Starting infrastructure...");
    start_infrastructure(&config, &manager, &work_dir, &log_dir).await?;

    // Phase 2: Start coordinator
    info!("🏗️  Phase 2: Starting coordinator...");
    start_coordinator(&config, &manager, &work_dir, &log_dir).await?;

    // Phase 3: Start realms
    info!("🏗️  Phase 3: Starting realms...");
    start_realms(&config, &manager, &work_dir, &log_dir).await?;

    // Phase 5: Start independent workers
    info!("🏗️  Phase 4: Starting independent workers...");
    start_independent_workers(&config, &manager, &work_dir, &log_dir).await?;

    // Phase 2: Start global services
    info!("🏗️  Phase 5: Starting global services...");
    start_global_services(&config, &manager, &work_dir, &log_dir).await?;


    info!("✅ All services started successfully!");
    info!("");
    info!("📡 Service endpoints:");

    // Display coordinator endpoint
    if let Some(coord_config) = config.network.coordinator_configs.first() {
        if let Some(url) = coord_config.rpc_url.first() {
            info!("  Coordinator RPC: {}", url);
        }
    }

    // Display realm endpoints
    for realm_config in &config.network.realm_configs {
        if let Some(url) = realm_config.rpc_url.first() {
            info!("  Realm {} RPC: {}", realm_config.id, url);
        }
    }

    info!("");
    info!("📊 Monitoring:");
    info!("  Logs: {}", log_dir.display());
    info!("");
    info!("Press Ctrl+C to stop all services...");

    // Wait for interrupt
    tokio::signal::ctrl_c().await?;

    info!("\n🛑 Shutting down...");
    manager.kill_all();

    // Stop Docker containers if any
    stop_docker_containers();

    Ok(())
}

async fn start_infrastructure(
    config: &Config,
    manager: &ProcessManager,
    work_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    // Extract Redis configuration
    let coordinator_redis = config.nodes.coordinator.redis.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Redis configuration found for coordinator"))?;
    let coordinator_uri = &coordinator_redis.uri;
    let coordinator_port = extract_port(coordinator_uri)?;

    // Start Redis instances
    if check_command_exists("redis-server") {
        // Coordinator Redis
        let mut cmd = Command::new("redis-server");
        cmd.arg("--port").arg(coordinator_port.to_string());
        manager.spawn("redis-coordinator".to_string(), cmd, log_dir)?;

        // Realm Redis instances
        for realm in &config.nodes.realms {
            if let Some(realm_redis) = &realm.redis {
                let port = extract_port(&realm_redis.uri)?;
                let mut cmd = Command::new("redis-server");
                cmd.arg("--port").arg(port.to_string());
                manager.spawn(format!("redis-realm-{}", realm.id), cmd, log_dir)?;
            }
        }
    } else {
        // Use Docker as fallback
        info!("Redis not found, using Docker containers...");
        start_redis_docker(config, manager, log_dir)?;
    }

    // Start ScyllaDB if needed
    // Check if any node is using scylla
    let has_scylla = config.nodes.coordinator.backend.as_ref()
        .map(|b| b.database == "scylla").unwrap_or(false) ||
        config.nodes.realms.iter().any(|r|
            r.backend.as_ref().map(|b| b.database == "scylla").unwrap_or(false));

    if has_scylla {
        start_scylla_docker(config, manager, log_dir)?;
        // Wait for ScyllaDB to be ready
        info!("⏳ Waiting for ScyllaDB to initialize...");
        sleep(Duration::from_secs(30)).await;
        create_keyspaces(config)?;
    }

    // Create LMDBX directories if needed for each component
    if let Some(coord_backend) = &config.nodes.coordinator.backend {
        if coord_backend.database == "lmdbx" {
            if let Some(lmdbx_cfg) = &coord_backend.lmdbx {
                fs::create_dir_all(&lmdbx_cfg.path)?;
            }
        }
    }

    for realm in &config.nodes.realms {
        if let Some(realm_backend) = &realm.backend {
            if realm_backend.database == "lmdbx" {
                if let Some(lmdbx_cfg) = &realm_backend.lmdbx {
                    fs::create_dir_all(&lmdbx_cfg.path)?;
                }
            }
        }
    }

    // Wait for infrastructure to be ready
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn start_coordinator(
    config: &Config,
    manager: &ProcessManager,
    work_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    let binary = get_binary_path()?;
    let coordinator_redis = config.nodes.coordinator.redis.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Redis configuration found for coordinator"))?;
    let redis_uri = &coordinator_redis.uri;
    let node_cfg = &config.nodes.coordinator;

    // Start processor
    if node_cfg.processor.enabled {
        let mut cmd = Command::new(&binary);
        cmd.arg("coordinator-processor");
        add_backend_args(&mut cmd, config, None)?;
        cmd.arg("--redis-uri").arg(redis_uri);
        add_service_args(&mut cmd, &node_cfg.processor)?;

        // Set environment variables
        for (key, value) in &node_cfg.processor.env {
            cmd.env(key, value);
        }

        manager.spawn("coordinator-processor".to_string(), cmd, log_dir)?;
    }

    // Start watcher
    if let Some(watcher_config) = &node_cfg.watcher {
        if watcher_config.enabled {
            let mut cmd = Command::new(&binary);
            cmd.arg("watcher");
            cmd.arg("--node-type").arg("coordinator");
            add_backend_args(&mut cmd, config, None)?;
            cmd.arg("--redis-url").arg(redis_uri);
            cmd.arg("--api-endpoint").arg("http://localhost:3000");
            add_service_args(&mut cmd, watcher_config)?;

            // Set environment variables
            for (key, value) in &watcher_config.env {
                cmd.env(key, value);
            }

            manager.spawn("coordinator-watcher".to_string(), cmd, log_dir)?;
        }
    }

    // Wait a bit before starting edge
    sleep(Duration::from_secs(1)).await;

    // Start edge
    if node_cfg.edge.enabled {
        let mut cmd = Command::new(&binary);
        cmd.arg("coordinator-edge");
        add_backend_args(&mut cmd, config, None)?;
        cmd.arg("--redis-uri").arg(redis_uri);
        add_service_args(&mut cmd, &node_cfg.edge)?;

        // Set environment variables
        for (key, value) in &node_cfg.edge.env {
            cmd.env(key, value);
        }

        manager.spawn("coordinator-edge".to_string(), cmd, log_dir)?;

        // Wait for coordinator to be ready
        if let Some(listen_addr) = node_cfg.edge.args.get("listen_addr") {
            if let Value::String(addr) = listen_addr {
                let port = extract_port_from_addr(&addr)?;
                wait_for_service("localhost", port, "Coordinator").await?;
            }
        }
    }

    Ok(())
}

async fn start_realms(
    config: &Config,
    manager: &ProcessManager,
    work_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    let binary = get_binary_path()?;

    // Get coordinator URL
    let coordinator_url = if let Some(coord_cfg) = config.network.coordinator_configs.first() {
        coord_cfg.rpc_url.first().cloned().unwrap_or_else(|| "http://127.0.0.1:8545".to_string())
    } else {
        "http://127.0.0.1:8545".to_string()
    };

    for realm_node in &config.nodes.realms {
        info!("🌍 Starting realm {}...", realm_node.id);

        // Find corresponding Redis config
        let realm_redis = realm_node.redis.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Redis config not found for realm {}", realm_node.id))?;

        let redis_uri = &realm_redis.uri;

        // Start processor
        if realm_node.processor.enabled {
            let mut cmd = Command::new(&binary);
            cmd.arg("realm-processor");
            add_backend_args(&mut cmd, config, Some(realm_node.id))?;
            cmd.arg("--redis-uri").arg(redis_uri);
            cmd.arg("--realm-id").arg(realm_node.id.to_string());
            add_service_args(&mut cmd, &realm_node.processor)?;

            // Set environment variables
            for (key, value) in &realm_node.processor.env {
                cmd.env(key, value);
            }

            manager.spawn(format!("realm-{}-processor", realm_node.id), cmd, log_dir)?;
        }

        // Start watcher
        if let Some(watcher_config) = &realm_node.watcher {
            if watcher_config.enabled {
                let mut cmd = Command::new(&binary);
                cmd.arg("watcher");
                cmd.arg("--node-type").arg("realm");
                add_backend_args(&mut cmd, config, Some(realm_node.id))?;
                cmd.arg("--redis-url").arg(redis_uri);
                cmd.arg("--api-endpoint").arg("http://localhost:3000");
                add_service_args(&mut cmd, watcher_config)?;

                // Set environment variables
                for (key, value) in &watcher_config.env {
                    cmd.env(key, value);
                }

                manager.spawn(format!("realm-{}-watcher", realm_node.id), cmd, log_dir)?;
            }
        }

        // Wait a bit before starting edge
        sleep(Duration::from_millis(500)).await;

        // Start edge
        if realm_node.edge.enabled {
            let mut cmd = Command::new(&binary);
            cmd.arg("realm-edge");
            add_backend_args(&mut cmd, config, Some(realm_node.id))?;
            cmd.arg("--redis-uri").arg(redis_uri);
            cmd.arg("--realm-id").arg(realm_node.id.to_string());

            // Add coordinator address if specified
            if let Some(coord_addr) = realm_node.edge.args.get("coordinator_addr") {
                if let Value::String(addr) = coord_addr {
                    cmd.arg("--coordinator-addr").arg(addr);
                }
            } else {
                cmd.arg("--coordinator-addr").arg(&coordinator_url);
            }

            add_service_args(&mut cmd, &realm_node.edge)?;

            // Set environment variables
            for (key, value) in &realm_node.edge.env {
                cmd.env(key, value);
            }

            manager.spawn(format!("realm-{}-edge", realm_node.id), cmd, log_dir)?;

            // Wait for realm to be ready
            if let Some(listen_addr) = realm_node.edge.args.get("listen_addr") {
                if let Value::String(addr) = listen_addr {
                    let port = extract_port_from_addr(&addr)?;
                    wait_for_service("localhost", port, &format!("Realm {}", realm_node.id)).await?;
                }
            }
        }
    }

    Ok(())
}

fn add_backend_args(cmd: &mut Command, config: &Config, realm_id: Option<u64>) -> Result<()> {
    let backend = if let Some(realm) = realm_id {
        config.nodes.realms.iter()
            .find(|r| r.id == realm)
            .and_then(|r| r.backend.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No backend configuration found for realm {}", realm))?
    } else {
        config.nodes.coordinator.backend.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No backend configuration found for coordinator"))?
    };

    match backend.database.as_str() {
        "lmdbx" => {
            cmd.arg("--database").arg("lmdbx");
            if let Some(lmdbx_cfg) = &backend.lmdbx {
                cmd.arg("--lmdbx-path").arg(&lmdbx_cfg.path);
            }
        }
        "scylla" => {
            cmd.arg("--database").arg("scylla");
            if let Some(scylla_cfg) = &backend.scylla {
                if let Some(endpoint) = scylla_cfg.endpoints.first() {
                    cmd.arg("--scylla-uri").arg(endpoint);
                }
                let keyspace = if let Some(realm) = realm_id {
                    format!("qed_realm_{}", realm)
                } else {
                    "qed_coordinator".to_string()
                };
                cmd.arg("--scylla-keyspace").arg(keyspace);
            }
        }
        _ => return Err(anyhow::anyhow!("Unknown backend type: {}", backend.database)),
    }
    Ok(())
}

fn add_service_args(cmd: &mut Command, service: &ServiceConfig) -> Result<()> {
    for (key, value) in &service.args {
        let flag = format!("--{}", key.replace('_', "-"));
        match value {
            Value::String(s) => cmd.arg(&flag).arg(s),
            Value::Number(n) => cmd.arg(&flag).arg(n.to_string()),
            Value::Bool(b) => {
                if *b {
                    cmd.arg(&flag)
                } else {
                    continue
                }
            },
            _ => continue,
        };
    }
    Ok(())
}

fn get_binary_path() -> Result<PathBuf> {
    // First check if binary exists in target/release
    let release_path = PathBuf::from("./target/release/psy_node_cli");
    if release_path.exists() {
        return Ok(release_path);
    }

    // Then check debug
    let debug_path = PathBuf::from("./target/debug/psy_node_cli");
    if debug_path.exists() {
        return Ok(debug_path);
    }

    // Finally check if it's in PATH
    if let Ok(output) = Command::new("which").arg("psy_node_cli").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(PathBuf::from(path));
        }
    }

    Err(anyhow::anyhow!(
        "psy_node_cli binary not found. Please build it first with 'cargo build --release'"
    ))
}

fn check_command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn start_redis_docker(config: &Config, manager: &ProcessManager, log_dir: &Path) -> Result<()> {
    // Coordinator Redis
    let coordinator_redis = config.nodes.coordinator.redis.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Redis configuration found for coordinator"))?;
    let coordinator_port = extract_port(&coordinator_redis.uri)?;
    let mut cmd = Command::new("docker");
    cmd.args(&[
        "run", "--rm", "--name", "qed-redis-coordinator",
        "-p", &format!("{}:6379", coordinator_port),
        "redis:7-alpine"
    ]);
    manager.spawn("docker-redis-coordinator".to_string(), cmd, log_dir)?;

    // Realm Redis instances
    for realm in &config.nodes.realms {
        if let Some(realm_redis) = &realm.redis {
            let port = extract_port(&realm_redis.uri)?;
            let mut cmd = Command::new("docker");
            cmd.args(&[
                "run", "--rm", "--name", &format!("qed-redis-realm-{}", realm.id),
                "-p", &format!("{}:6379", port),
                "redis:7-alpine"
            ]);
            manager.spawn(format!("docker-redis-realm-{}", realm.id), cmd, log_dir)?;
        }
    }

    Ok(())
}

fn start_scylla_docker(config: &Config, manager: &ProcessManager, log_dir: &Path) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.args(&[
        "run", "--rm", "--name", "qed-scylladb",
        "-p", "9042:9042",
        "scylladb/scylla:2025.1",
        "--smp", "2",
        "--memory", "4G"
    ]);
    manager.spawn("docker-scylladb".to_string(), cmd, log_dir)?;
    Ok(())
}

fn create_keyspaces(config: &Config) -> Result<()> {
    let mut keyspaces = vec!["qed_coordinator".to_string()];
    let realm_keyspaces: Vec<String> = config.nodes.realms.iter()
        .map(|r| format!("qed_realm_{}", r.id))
        .collect();
    keyspaces.extend(realm_keyspaces);

    for keyspace in &keyspaces {
        info!("Creating keyspace: {}", keyspace);
        let output = Command::new("docker")
            .args(&[
                "exec", "qed-scylladb", "cqlsh", "-e",
                &format!(
                    "CREATE KEYSPACE IF NOT EXISTS {} WITH replication = {{'class': 'SimpleStrategy', 'replication_factor': 1}};",
                    keyspace
                )
            ])
            .output()?;

        if !output.status.success() {
            warn!("Failed to create keyspace {}: {}", keyspace, String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
async fn start_independent_workers(
    config: &Config,
    manager: &ProcessManager,
    work_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    if let Some(workers) = &config.nodes.workers {
        if workers.enabled {
            let binary = get_binary_path()?;

            info!("👷 Starting workers...");

            // First, ensure config.json exists with correct RPC URLs
            ensure_worker_config_file(config, work_dir)?;

            // Get task count from AWS config or default to 3
            let task_count = workers.aws.as_ref()
                .and_then(|aws| aws.ecs.as_ref())
                .map(|ecs| ecs.task_count)
                .unwrap_or(3);

            info!("Starting {} worker tasks", task_count);

            for instance_idx in 0..task_count {
                let mut cmd = Command::new(&binary);
                cmd.arg("worker");

                // Workers use config.json to discover nodes
                cmd.arg("--config").arg(work_dir.join("config.json"));

                // Add worker args (private-key, keystore-path, etc.)
                for (key, value) in &workers.args {
                    // Skip config since we already set it above
                    if key == "config" {
                        continue;
                    }

                    cmd.arg(format!("--{}", key.replace('_', "-")));

                    match value {
                        Value::String(s) => {
                            cmd.arg(s);
                        },
                        Value::Number(n) => {
                            cmd.arg(n.to_string());
                        },
                        Value::Bool(b) => {
                            cmd.arg(b.to_string());
                        },
                        _ => {}
                    }
                }

                // Set environment variables
                for (key, value) in &workers.env {
                    cmd.env(key, value);
                }

                // Add API service URL to environment for worker reporting
                cmd.env("API_SERVICE_URL", "http://localhost:3000");

                let service_name = format!("worker-{}", instance_idx);
                manager.spawn(service_name, cmd, log_dir)?;
            }
        }
    }

    Ok(())
}

// Helper function to ensure config.json has correct RPC URLs for workers
pub fn ensure_worker_config_file(config: &Config, work_dir: &Path) -> Result<()> {
    info!("Creating config.json for workers with RPC endpoints...");

    // Build the network configuration that workers need
    let worker_config = json!({
        "network": {
            "coordinator_configs": config.network.coordinator_configs.iter().map(|c| {
                json!({
                    "id": c.id,
                    "rpc_url": c.rpc_url
                })
            }).collect::<Vec<_>>(),
            "realm_configs": config.network.realm_configs.iter().map(|r| {
                json!({
                    "id": r.id,
                    "rpc_url": r.rpc_url
                })
            }).collect::<Vec<_>>(),
            "prove_proxy_url": config.network.prove_proxy_url,
            "native_currency": config.network.native_currency,
        }
    });

    let config_path = work_dir.join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&worker_config)?)
        .with_context(|| format!("Failed to write worker config to {}", config_path.display()))?;

    info!("Worker config written to: {}", config_path.display());
    info!("  Coordinator RPC: {:?}", config.network.coordinator_configs[0].rpc_url);
    for realm in &config.network.realm_configs {
        info!("  Realm {} RPC: {:?}", realm.id, realm.rpc_url);
    }

    Ok(())
}

async fn start_global_services(
    config: &Config,
    manager: &ProcessManager,
    work_dir: &Path,
    log_dir: &Path,
) -> Result<()> {
    if let Some(global_services) = &config.global_api_services {
        info!("🌐 Starting global services...");

        // Start TimescaleDB
        if let Some(timescale_config) = &global_services.timescaledb {
            if timescale_config.enabled {
                info!("🐘 Starting TimescaleDB...");
                let mut cmd = Command::new("docker");
                cmd.args(&[
                    "run", "--rm", "--name", "qed-timescaledb",
                    "-p", "5432:5432",
                    "-e", "POSTGRES_PASSWORD=password",
                    "timescale/timescaledb:latest-pg17"
                ]);
                manager.spawn("docker-timescaledb".to_string(), cmd, log_dir)?;

                // Wait for TimescaleDB to be ready
                sleep(Duration::from_secs(10)).await;

                // Run database migrations
                info!("🔄 Running database migrations...");
                let mut migrate_cmd = Command::new("cargo");
                migrate_cmd.args(&["run", "--bin", "qed_api_services", "--", "migrate"])
                    .current_dir("./qed_api_services")
                    .env("DATABASE_URL", "postgres://postgres:password@localhost/postgres");

                let output = migrate_cmd.output()?;
                if !output.status.success() {
                    warn!("Migration may have failed: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }

        // Start API Service
        if let Some(api_service) = &global_services.api_service {
            if api_service.enabled {
                info!("🔌 Starting API Service...");
                let mut cmd = Command::new("./target/release/qed_api_services");

                // Set default environment variables
                cmd.env("DATABASE_URL", "postgres://postgres:password@localhost/postgres");

                // Add service-specific args
                add_service_args(&mut cmd, api_service)?;

                // Set environment variables
                for (key, value) in &api_service.env {
                    cmd.env(key, value);
                }

                manager.spawn("qed-api-service".to_string(), cmd, log_dir)?;

                // Wait for API service to be ready
                wait_for_service("localhost", 3000, "API Service").await?;
            }
        }
    }

    Ok(())
}
fn extract_port(uri: &str) -> Result<u16> {
    let parts: Vec<&str> = uri.split(':').collect();
    if parts.len() >= 3 {
        parts[2].parse::<u16>()
            .with_context(|| format!("Failed to parse port from URI: {}", uri))
    } else {
        Err(anyhow::anyhow!("Invalid URI format: {}", uri))
    }
}

fn extract_port_from_addr(addr: &str) -> Result<u16> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() >= 2 {
        parts[1].parse::<u16>()
            .with_context(|| format!("Failed to parse port from address: {}", addr))
    } else {
        Err(anyhow::anyhow!("Invalid address format: {}", addr))
    }
}

fn stop_docker_containers() {
    let containers = vec![
        "qed-redis-coordinator",
        "qed-redis-realm-0",
        "qed-redis-realm-1",
        "qed-scylladb",
        "qed-timescaledb"

    ];

    for container in containers {
        let _ = Command::new("docker")
            .args(&["stop", container])
            .output();
        let _ = Command::new("docker")
            .args(&["rm", container])
            .output();
    }
}

async fn wait_for_service(host: &str, port: u16, name: &str) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 30;

    loop {
        match tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
            Ok(_) => {
                info!("✅ {} is ready", name);
                return Ok(());
            }
            Err(_) if attempts < max_attempts => {
                attempts += 1;
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("{} failed to start on port {}: {}", name, port, e));
            }
        }
    }
}

