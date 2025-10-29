use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    Edge,
    Worker,
    Processor,
    Redis,
    ScyllaDB,
    Prover,
    Watcher,
    ApiService,
    TiKV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceFamily {
    GeneralPurpose,   // m5/m6i
    ComputeOptimized, // c5/c6i
    MemoryOptimized,  // r5/r6i
    StorageOptimized, // i3/i3en
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleInstance {
    pub name: String,
    pub family: InstanceFamily,
    pub vcpus: u32,
    pub memory_gb: f32,
    pub price_per_hour: f32,
}

#[derive(Debug, Clone)]
pub struct ServiceGroupRequirements {
    pub name: String,
    pub service_type: ServiceType,
    pub total_vcpus: u32,
    pub total_memory_gb: f32,
    pub instance_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimpleInstanceRecommendation {
    pub group_name: String,
    pub service_type: ServiceType,
    pub instance_type: SimpleInstance,
    pub instance_count: u32,
    pub total_vcpus: u32,
    pub total_memory_gb: f32,
    pub hourly_cost: f32,
    pub monthly_cost: f32,
}

pub struct SimpleInstanceSelector {
    instances: Vec<SimpleInstance>,
    resource_margin: f32,
}

impl SimpleInstanceSelector {
    pub fn new() -> Self {
        Self {
            instances: Self::load_instances(),
            resource_margin: 0.25, // 25% 余量
        }
    }

    fn load_instances() -> Vec<SimpleInstance> {
        vec![
            // M6i Series - Universal Balanced (Edge Service) - Latest Generation
            SimpleInstance {
                name: "m6i.large".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 2,
                memory_gb: 8.0,
                price_per_hour: 0.096,
            },
            SimpleInstance {
                name: "m6i.xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 4,
                memory_gb: 16.0,
                price_per_hour: 0.192,
            },
            SimpleInstance {
                name: "m6i.2xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 8,
                memory_gb: 32.0,
                price_per_hour: 0.384,
            },
            SimpleInstance {
                name: "m6i.4xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 16,
                memory_gb: 64.0,
                price_per_hour: 0.768,
            },
            // C5/C6i 系列 - 计算优化型 (Worker/Processor/Prover)
            SimpleInstance {
                name: "c5.large".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 2,
                memory_gb: 4.0,
                price_per_hour: 0.085,
            },
            SimpleInstance {
                name: "c5.xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 4,
                memory_gb: 8.0,
                price_per_hour: 0.17,
            },
            SimpleInstance {
                name: "c5.2xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 8,
                memory_gb: 16.0,
                price_per_hour: 0.34,
            },
            SimpleInstance {
                name: "c5.4xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 16,
                memory_gb: 32.0,
                price_per_hour: 0.68,
            },
            SimpleInstance {
                name: "c6i.large".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 2,
                memory_gb: 4.0,
                price_per_hour: 0.085,
            },
            SimpleInstance {
                name: "c6i.xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 4,
                memory_gb: 8.0,
                price_per_hour: 0.17,
            },
            SimpleInstance {
                name: "c6i.2xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 8,
                memory_gb: 16.0,
                price_per_hour: 0.34,
            },
            SimpleInstance {
                name: "c6i.4xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 16,
                memory_gb: 32.0,
                price_per_hour: 0.68,
            },
            // R5/R6i 系列 - 内存优化型 (Redis)
            SimpleInstance {
                name: "r5.large".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 2,
                memory_gb: 16.0,
                price_per_hour: 0.126,
            },
            SimpleInstance {
                name: "r5.xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 4,
                memory_gb: 32.0,
                price_per_hour: 0.252,
            },
            SimpleInstance {
                name: "r5.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.504,
            },
            SimpleInstance {
                name: "r6i.large".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 2,
                memory_gb: 16.0,
                price_per_hour: 0.126,
            },
            SimpleInstance {
                name: "r6i.xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 4,
                memory_gb: 32.0,
                price_per_hour: 0.252,
            },
            SimpleInstance {
                name: "r6i.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.504,
            },
            // I3/I3en 系列 - 存储优化型 (ScyllaDB)
            SimpleInstance {
                name: "i3.large".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 2,
                memory_gb: 15.25,
                price_per_hour: 0.156,
            },
            SimpleInstance {
                name: "i3.xlarge".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 4,
                memory_gb: 30.5,
                price_per_hour: 0.312,
            },
            SimpleInstance {
                name: "i3.2xlarge".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 8,
                memory_gb: 61.0,
                price_per_hour: 0.624,
            },
            SimpleInstance {
                name: "i3en.large".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 2,
                memory_gb: 16.0,
                price_per_hour: 0.226,
            },
            SimpleInstance {
                name: "i3en.xlarge".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 4,
                memory_gb: 32.0,
                price_per_hour: 0.452,
            },
            SimpleInstance {
                name: "i3en.2xlarge".to_string(),
                family: InstanceFamily::StorageOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.904,
            },
        ]
    }

    fn get_preferred_family(service_type: ServiceType) -> InstanceFamily {
        match service_type {
            ServiceType::Edge => InstanceFamily::GeneralPurpose,
            ServiceType::Worker | ServiceType::Processor | ServiceType::Prover => InstanceFamily::ComputeOptimized,
            ServiceType::Redis => InstanceFamily::MemoryOptimized,
            ServiceType::ScyllaDB => InstanceFamily::StorageOptimized,
            ServiceType::Watcher => InstanceFamily::GeneralPurpose,
            ServiceType::ApiService => InstanceFamily::GeneralPurpose,
            ServiceType::TiKV => InstanceFamily::StorageOptimized,
        }
    }

    pub fn calculate_recommendation(&self, requirements: &ServiceGroupRequirements) -> Result<SimpleInstanceRecommendation> {
        let preferred_family = Self::get_preferred_family(requirements.service_type);

        // 应用25%余量
        let required_vcpus = (requirements.total_vcpus as f32 * (1.0 + self.resource_margin)).ceil() as u32;
        let required_memory = requirements.total_memory_gb * (1.0 + self.resource_margin);

        debug!(
            "Calculating {} service group instance recommendation: {} vCPUs, {:.1} GB memory (with {}% margin)",
            requirements.name,
            required_vcpus,
            required_memory,
            (self.resource_margin * 100.0) as u32
        );

        // Filter for appropriate instance types (giving priority to specific series)
        let preferred_instances: Vec<&SimpleInstance> = self.instances.iter().filter(|instance| instance.family == preferred_family).collect();

        // If there is no suitable instance for the preferred series, consider all
        // instances
        let candidate_instances: Vec<&SimpleInstance> = if preferred_instances.is_empty() {
            self.instances.iter().collect()
        } else {
            preferred_instances
        };

        // Find the most suitable instance
        let mut best_option: Option<(SimpleInstance, u32, f32)> = None;

        for instance in candidate_instances {
            // Calculate how many instances are needed
            let instances_for_cpu = (required_vcpus as f32 / instance.vcpus as f32).ceil() as u32;
            let instances_for_memory = (required_memory / instance.memory_gb).ceil() as u32;
            let instances_needed = instances_for_cpu.max(instances_for_memory);

            // Limit the number of instances (to avoid management complexity)
            if instances_needed > 10 {
                continue;
            }

            // Check if the requirements are met
            let total_vcpus = instance.vcpus * instances_needed;
            let total_memory = instance.memory_gb * instances_needed as f32;

            if total_vcpus >= required_vcpus && total_memory >= required_memory {
                let total_cost = instance.price_per_hour * instances_needed as f32;

                // Simple cost efficiency score (resource utilization / cost)
                let cpu_utilization = requirements.total_vcpus as f32 / total_vcpus as f32;
                let memory_utilization = requirements.total_memory_gb / total_memory;
                let avg_utilization = (cpu_utilization + memory_utilization) / 2.0;
                let efficiency_score = avg_utilization / total_cost;

                if best_option.is_none() || efficiency_score > best_option.as_ref().unwrap().2 {
                    best_option = Some((instance.clone(), instances_needed, efficiency_score));
                }
            }
        }

        let (best_instance, instance_count, _) = best_option.ok_or_else(|| {
            anyhow::anyhow!(
                "Unable to find a suitable instance type for {}: requires {} vCPUs, {:.1} GB memory",
                requirements.name,
                required_vcpus,
                required_memory
            )
        })?;

        Ok(SimpleInstanceRecommendation {
            group_name: requirements.name.clone(),
            service_type: requirements.service_type,
            instance_type: best_instance.clone(),
            instance_count,
            total_vcpus: best_instance.vcpus * instance_count,
            total_memory_gb: best_instance.memory_gb * instance_count as f32,
            hourly_cost: best_instance.price_per_hour * instance_count as f32,
            monthly_cost: best_instance.price_per_hour * instance_count as f32 * 24.0 * 30.0,
        })
    }

    pub fn calculate_multiple_recommendations(&self, requirements: Vec<ServiceGroupRequirements>) -> Result<Vec<SimpleInstanceRecommendation>> {
        let mut recommendations = Vec::new();

        for req in requirements {
            let recommendation = self.calculate_recommendation(&req)?;
            recommendations.push(recommendation);
        }

        Ok(recommendations)
    }

    pub fn print_recommendations_summary(recommendations: &[SimpleInstanceRecommendation]) {
        if recommendations.is_empty() {
            info!("No instance recommendations");
            return;
        }

        info!("\n=== Instance Recommendation Summary ===");

        let mut total_instances = 0;
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut total_hourly_cost = 0.0;

        // Group statistics by instance type
        let mut instance_counts: HashMap<String, u32> = HashMap::new();

        for rec in recommendations {
            info!("\n{} ({:?}):", rec.group_name, rec.service_type);
            info!("  Recommended instance: {} x {}", rec.instance_count, rec.instance_type.name);
            info!("  Total resources: {} vCPUs, {:.1} GB RAM", rec.total_vcpus, rec.total_memory_gb);
            info!("  Cost: ${:.2}/hour, ${:.2}/month", rec.hourly_cost, rec.monthly_cost);

            total_instances += rec.instance_count;
            total_vcpus += rec.total_vcpus;
            total_memory_gb += rec.total_memory_gb;
            total_hourly_cost += rec.hourly_cost;

            *instance_counts.entry(rec.instance_type.name.clone()).or_insert(0) += rec.instance_count;
        }

        info!("\n--- Total ---");
        info!("Total instances: {}", total_instances);
        info!("Total vCPUs: {}", total_vcpus);
        info!("Total memory: {:.1} GB", total_memory_gb);
        info!(
            "Total cost: ${:.2}/hour, ${:.2}/month",
            total_hourly_cost,
            total_hourly_cost * 24.0 * 30.0
        );

        info!("\nInstance type distribution:");
        let mut types: Vec<_> = instance_counts.iter().collect();
        types.sort_by_key(|(name, _)| name.as_str());
        for (instance_type, count) in types {
            info!("  {}: {} instances", instance_type, count);
        }
    }

    /// Build service group requirements from configuration
    pub fn build_service_requirements_from_config(config: &crate::subcommand::generate::Config) -> Vec<ServiceGroupRequirements> {
        let mut requirements = Vec::new();

        if let Some(coordinator_req) = Self::build_coordinator_requirements(config) {
            requirements.push(coordinator_req);
        }

        for realm in &config.nodes.realms {
            if let Some(realm_req) = Self::build_realm_requirements(config, realm) {
                requirements.push(realm_req);
            }
        }

        if let Some(redis_req) = Self::build_redis_requirements(config) {
            requirements.push(redis_req);
        }

        // Add independent workers
        if let Some(workers_req) = Self::build_independent_workers_requirements(config) {
            requirements.push(workers_req);
        }

        // Add global services
        if let Some(api_service_req) = Self::build_api_service_requirements(config) {
            requirements.push(api_service_req);
        }

        // Add TiKV requirements
        if let Some(tikv_req) = Self::build_tikv_requirements(config) {
            requirements.push(tikv_req);
        }

        requirements
    }

    fn build_coordinator_requirements(config: &crate::subcommand::generate::Config) -> Option<ServiceGroupRequirements> {
        let coordinator = &config.nodes.coordinator;
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut instance_count = 0;

        // Collect resource requirements for all enabled services
        if coordinator.processor.enabled {
            if let Some(aws_config) = &coordinator.processor.aws {
                // Get task count based on deployment type
                let task_count = match &aws_config.deployment_type {
                    Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                    Some(crate::subcommand::generate::DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
                    _ => 1,
                };
                total_vcpus += aws_config.cpu * task_count / 1024; // Convert to vCPUs
                total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
                // Convert to GB
            }
        }

        if coordinator.edge.enabled {
            if let Some(aws_config) = &coordinator.edge.aws {
                let task_count = match &aws_config.deployment_type {
                    Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                    Some(crate::subcommand::generate::DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
                    _ => 1,
                };
                total_vcpus += aws_config.cpu * task_count / 1024;
                total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
            }
        }

        // Add watcher if enabled
        if let Some(watcher) = &coordinator.watcher {
            if watcher.enabled {
                if let Some(aws_config) = &watcher.aws {
                    let task_count = match &aws_config.deployment_type {
                        Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                        Some(crate::subcommand::generate::DeploymentType::EC2) => {
                            aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1)
                        }
                        _ => 1,
                    };
                    total_vcpus += aws_config.cpu * task_count / 1024;
                    total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
                }
            }
        }

        // For EC2 instances, we want a reasonable minimum count
        instance_count = 1;

        if total_vcpus > 0 {
            Some(ServiceGroupRequirements {
                name: "Coordinator".to_string(),
                service_type: ServiceType::Worker, // 改为Worker类型以使用计算优化型实例
                total_vcpus,
                total_memory_gb,
                instance_count,
            })
        } else {
            None
        }
    }

    fn build_realm_requirements(
        config: &crate::subcommand::generate::Config,
        realm: &crate::subcommand::generate::RealmNode,
    ) -> Option<ServiceGroupRequirements> {
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut instance_count = 0;

        if realm.processor.enabled {
            if let Some(aws_config) = &realm.processor.aws {
                let task_count = match &aws_config.deployment_type {
                    Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                    Some(crate::subcommand::generate::DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
                    _ => 1,
                };
                total_vcpus += aws_config.cpu * task_count / 1024;
                total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
            }
        }

        if realm.edge.enabled {
            if let Some(aws_config) = &realm.edge.aws {
                let task_count = match &aws_config.deployment_type {
                    Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                    Some(crate::subcommand::generate::DeploymentType::EC2) => aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1),
                    _ => 1,
                };
                total_vcpus += aws_config.cpu * task_count / 1024;
                total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
            }
        }
        // Add watcher if enabled
        if let Some(watcher) = &realm.watcher {
            if watcher.enabled {
                if let Some(aws_config) = &watcher.aws {
                    let task_count = match &aws_config.deployment_type {
                        Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                        Some(crate::subcommand::generate::DeploymentType::EC2) => {
                            aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1)
                        }
                        _ => 1,
                    };
                    total_vcpus += aws_config.cpu * task_count / 1024;
                    total_memory_gb += aws_config.memory as f32 * task_count as f32 / 1024.0;
                }
            }
        }

        // For EC2 instances, we want a reasonable minimum count
        instance_count = 1;

        if total_vcpus > 0 {
            Some(ServiceGroupRequirements {
                name: format!("Realm-{}", realm.id),
                service_type: ServiceType::Worker,
                total_vcpus,
                total_memory_gb,
                instance_count,
            })
        } else {
            None
        }
    }

    fn build_redis_requirements(config: &crate::subcommand::generate::Config) -> Option<ServiceGroupRequirements> {
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut instance_count = 0;

        // Check coordinator Redis
        if let Some(redis) = &config.nodes.coordinator.redis {
            if let Some(redis_aws) = &redis.aws {
                if let Some(crate::subcommand::generate::RedisDeploymentType::EC2) = &redis_aws.deployment_type {
                    if let (Some(cpu), Some(memory), Some(ec2)) = (&redis_aws.cpu, &redis_aws.memory, &redis_aws.ec2) {
                        let instances = ec2.desired_instances;
                        total_vcpus += cpu * instances / 1024;
                        total_memory_gb += *memory as f32 * instances as f32 / 1024.0;
                        instance_count += instances;
                    }
                }
            }
        }

        // Check realm Redis
        for realm in &config.nodes.realms {
            if let Some(redis) = &realm.redis {
                if let Some(redis_aws) = &redis.aws {
                    if let Some(crate::subcommand::generate::RedisDeploymentType::EC2) = &redis_aws.deployment_type {
                        if let (Some(cpu), Some(memory), Some(ec2)) = (&redis_aws.cpu, &redis_aws.memory, &redis_aws.ec2) {
                            let instances = ec2.desired_instances;
                            total_vcpus += cpu * instances / 1024;
                            total_memory_gb += *memory as f32 * instances as f32 / 1024.0;
                            instance_count += instances;
                        }
                    }
                }
            }
        }

        if total_vcpus > 0 {
            Some(ServiceGroupRequirements {
                name: "Redis".to_string(),
                service_type: ServiceType::Worker, // Use Worker type for Redis
                total_vcpus,
                total_memory_gb,
                instance_count,
            })
        } else {
            None
        }
    }
    fn build_independent_workers_requirements(config: &crate::subcommand::generate::Config) -> Option<ServiceGroupRequirements> {
        if let Some(workers) = &config.nodes.workers {
            if workers.enabled {
                if let Some(aws_config) = &workers.aws {
                    let task_count = match &aws_config.deployment_type {
                        Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(3),
                        Some(crate::subcommand::generate::DeploymentType::EC2) => {
                            aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(3)
                        }
                        _ => 3, // Default to 3 tasks
                    };

                    let total_vcpus = aws_config.cpu * task_count / 1024;
                    let total_memory_gb = aws_config.memory as f32 * task_count as f32 / 1024.0;

                    if total_vcpus > 0 {
                        return Some(ServiceGroupRequirements {
                            name: "Workers".to_string(),
                            service_type: ServiceType::Worker,
                            total_vcpus,
                            total_memory_gb,
                            instance_count: task_count,
                        });
                    }
                }
            }
        }
        None
    }

    fn build_api_service_requirements(config: &crate::subcommand::generate::Config) -> Option<ServiceGroupRequirements> {
        if let Some(global_services) = &config.global_api_services {
            if let Some(api_service) = &global_services.api_service {
                if api_service.enabled {
                    if let Some(aws_config) = &api_service.aws {
                        let task_count = match &aws_config.deployment_type {
                            Some(crate::subcommand::generate::DeploymentType::ECS) => aws_config.ecs.as_ref().map(|ecs| ecs.task_count).unwrap_or(1),
                            Some(crate::subcommand::generate::DeploymentType::EC2) => {
                                aws_config.ec2.as_ref().map(|ec2| ec2.desired_instances).unwrap_or(1)
                            }
                            _ => 1,
                        };

                        let total_vcpus = aws_config.cpu * task_count / 1024;
                        let total_memory_gb = aws_config.memory as f32 * task_count as f32 / 1024.0;

                        if total_vcpus > 0 {
                            return Some(ServiceGroupRequirements {
                                name: "API-Service".to_string(),
                                service_type: ServiceType::ApiService,
                                total_vcpus,
                                total_memory_gb,
                                instance_count: task_count,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    fn build_tikv_requirements(config: &crate::subcommand::generate::Config) -> Option<ServiceGroupRequirements> {
        // Check if any node is using TiKV
        let coordinator_uses_tikv = config.nodes.coordinator.backend.as_ref().map(|b| b.database == "tikv").unwrap_or(false);
        let realm_uses_tikv = config
            .nodes
            .realms
            .iter()
            .any(|r| r.backend.as_ref().map(|b| b.database == "tikv").unwrap_or(false));

        if coordinator_uses_tikv || realm_uses_tikv {
            // Get TiKV configuration from coordinator (assume unified TiKV cluster)
            if let Some(backend) = &config.nodes.coordinator.backend {
                if let Some(tikv_config) = &backend.tikv {
                    if let Some(tikv_aws) = &tikv_config.aws {
                        let tikv_instances = tikv_aws.tikv_count;
                        let pd_instances = tikv_aws.pd_count;
                        let total_instances = tikv_instances + pd_instances;

                        let total_vcpus = tikv_aws.cpu * total_instances / 1024;
                        let total_memory_gb = tikv_aws.memory as f32 * total_instances as f32 / 1024.0;

                        if total_vcpus > 0 {
                            return Some(ServiceGroupRequirements {
                                name: "TiKV-Cluster".to_string(),
                                service_type: ServiceType::TiKV,
                                total_vcpus,
                                total_memory_gb,
                                instance_count: total_instances,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

impl Default for SimpleInstanceSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_mapping() {
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::Edge),
            InstanceFamily::GeneralPurpose
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::Worker),
            InstanceFamily::ComputeOptimized
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::Redis),
            InstanceFamily::MemoryOptimized
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::ScyllaDB),
            InstanceFamily::StorageOptimized
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::Watcher),
            InstanceFamily::GeneralPurpose
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::ApiService),
            InstanceFamily::GeneralPurpose
        );
        assert_eq!(
            SimpleInstanceSelector::get_preferred_family(ServiceType::TiKV),
            InstanceFamily::StorageOptimized
        );
    }

    #[test]
    fn test_instance_recommendation() {
        let selector = SimpleInstanceSelector::new();

        let requirements = ServiceGroupRequirements {
            name: "test-group".to_string(),
            service_type: ServiceType::Worker,
            total_vcpus: 4,
            total_memory_gb: 8.0,
            instance_count: 2,
        };

        let recommendation = selector.calculate_recommendation(&requirements).unwrap();

        // Verify that the recommendation result contains sufficient resources
        // (including 25% margin)
        assert!(recommendation.total_vcpus >= 5); // 4 * 1.25 = 5
        assert!(recommendation.total_memory_gb >= 10.0); // 8.0 * 1.25 = 10.0
        assert_eq!(recommendation.service_type, ServiceType::Worker);
    }
}
