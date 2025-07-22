use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// EC2 instance type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceType {
    pub name: String,
    pub family: InstanceFamily,
    pub vcpus: u32,
    pub memory_gb: f32,
    pub price_per_hour: f32,
    pub network_performance: NetworkPerformance,
    pub ebs_optimized: bool,
}

/// Instance family classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceFamily {
    GeneralPurpose,  // m5, m6i
    ComputeOptimized, // c5, c6i
    MemoryOptimized,  // r5, r6i
}

/// Network performance tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkPerformance {
    Low,
    Moderate,
    High,
    VeryHigh,
    #[serde(rename = "10Gigabit")]
    TenGigabit,
    #[serde(rename = "25Gigabit")]
    TwentyFiveGigabit,
}

/// Optimization strategy for instance selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationStrategy {
    CostOptimized,
    PerformanceOptimized,
    Balanced,
}

/// Service group resource requirements
#[derive(Debug, Clone)]
pub struct ServiceGroupRequirements {
    pub name: String,
    pub total_vcpus: u32,
    pub total_memory_gb: f32,
    pub instances_count: u32,
    pub network_intensive: bool,
}

/// Instance recommendation
#[derive(Debug, Clone, Serialize)]
pub struct InstanceRecommendation {
    pub instance_type: InstanceType,
    pub instance_count: u32,
    pub total_vcpus: u32,
    pub total_memory_gb: f32,
    pub cpu_utilization: f32,
    pub memory_utilization: f32,
    pub hourly_cost: f32,
    pub monthly_cost: f32,
    pub recommendation_reason: String,
}

/// Instance calculator
pub struct InstanceCalculator {
    instance_types: Vec<InstanceType>,
    resource_margin: f32,
}

impl InstanceCalculator {
    /// Create a new instance calculator with default AWS instance types
    pub fn new() -> Self {
        Self {
            instance_types: Self::load_instance_types(),
            resource_margin: 0.25, // 25% margin by default
        }
    }

    /// Set resource margin (e.g., 0.3 for 30% margin)
    pub fn with_resource_margin(mut self, margin: f32) -> Self {
        self.resource_margin = margin;
        self
    }

    /// Load AWS instance type definitions
    fn load_instance_types() -> Vec<InstanceType> {
        vec![
            // M5 Series - General Purpose (Previous Generation)
            InstanceType {
                name: "m5.large".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 2,
                memory_gb: 8.0,
                price_per_hour: 0.096,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m5.xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 4,
                memory_gb: 16.0,
                price_per_hour: 0.192,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m5.2xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 8,
                memory_gb: 32.0,
                price_per_hour: 0.384,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m5.4xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 16,
                memory_gb: 64.0,
                price_per_hour: 0.768,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            
            // M6i Series - General Purpose (Current Generation)
            InstanceType {
                name: "m6i.large".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 2,
                memory_gb: 8.0,
                price_per_hour: 0.096,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m6i.xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 4,
                memory_gb: 16.0,
                price_per_hour: 0.192,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m6i.2xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 8,
                memory_gb: 32.0,
                price_per_hour: 0.384,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },
            InstanceType {
                name: "m6i.4xlarge".to_string(),
                family: InstanceFamily::GeneralPurpose,
                vcpus: 16,
                memory_gb: 64.0,
                price_per_hour: 0.768,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },

            // C5 Series - Compute Optimized (Previous Generation)
            InstanceType {
                name: "c5.large".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 2,
                memory_gb: 4.0,
                price_per_hour: 0.085,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c5.xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 4,
                memory_gb: 8.0,
                price_per_hour: 0.17,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c5.2xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 8,
                memory_gb: 16.0,
                price_per_hour: 0.34,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c5.4xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 16,
                memory_gb: 32.0,
                price_per_hour: 0.68,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },

            // C6i Series - Compute Optimized (Current Generation)
            InstanceType {
                name: "c6i.large".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 2,
                memory_gb: 4.0,
                price_per_hour: 0.085,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c6i.xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 4,
                memory_gb: 8.0,
                price_per_hour: 0.17,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c6i.2xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 8,
                memory_gb: 16.0,
                price_per_hour: 0.34,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },
            InstanceType {
                name: "c6i.4xlarge".to_string(),
                family: InstanceFamily::ComputeOptimized,
                vcpus: 16,
                memory_gb: 32.0,
                price_per_hour: 0.68,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },

            // R5 Series - Memory Optimized (Previous Generation)
            InstanceType {
                name: "r5.large".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 2,
                memory_gb: 16.0,
                price_per_hour: 0.126,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r5.xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 4,
                memory_gb: 32.0,
                price_per_hour: 0.252,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r5.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.504,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r5.4xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 16,
                memory_gb: 128.0,
                price_per_hour: 1.008,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },

            // R6i Series - Memory Optimized (Current Generation)
            InstanceType {
                name: "r6i.large".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 2,
                memory_gb: 16.0,
                price_per_hour: 0.126,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r6i.xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 4,
                memory_gb: 32.0,
                price_per_hour: 0.252,
                network_performance: NetworkPerformance::High,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r6i.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.504,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },
            InstanceType {
                name: "r6i.4xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 16,
                memory_gb: 128.0,
                price_per_hour: 1.008,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            },
        ]
    }

    /// Calculate recommended instance type for a service group
    pub fn calculate_recommendation(
        &self,
        requirements: &ServiceGroupRequirements,
        strategy: OptimizationStrategy,
    ) -> Result<InstanceRecommendation> {
        // Apply resource margin
        let required_vcpus = (requirements.total_vcpus as f32 * (1.0 + self.resource_margin)).ceil() as u32;
        let required_memory = requirements.total_memory_gb * (1.0 + self.resource_margin);

        info!(
            "Calculating instance recommendation for '{}': {} vCPUs, {:.1} GB memory (with {}% margin)",
            requirements.name,
            required_vcpus,
            required_memory,
            (self.resource_margin * 100.0) as u32
        );

        // Filter suitable instance types
        let mut suitable_instances: Vec<(InstanceType, u32)> = Vec::new();

        for instance_type in &self.instance_types {
            // Skip if network performance is insufficient for network-intensive workloads
            if requirements.network_intensive && 
               matches!(instance_type.network_performance, NetworkPerformance::Low | NetworkPerformance::Moderate) {
                continue;
            }

            // Calculate how many instances we need
            let instances_for_cpu = (required_vcpus as f32 / instance_type.vcpus as f32).ceil() as u32;
            let instances_for_memory = (required_memory / instance_type.memory_gb).ceil() as u32;
            let instances_needed = instances_for_cpu.max(instances_for_memory);

            // Skip if we need too many instances (complexity/management overhead)
            if instances_needed > 10 {
                continue;
            }

            // Check if this configuration meets requirements
            let total_vcpus = instance_type.vcpus * instances_needed;
            let total_memory = instance_type.memory_gb * instances_needed as f32;

            if total_vcpus >= required_vcpus && total_memory >= required_memory {
                suitable_instances.push((instance_type.clone(), instances_needed));
            }
        }

        if suitable_instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No suitable instance types found for requirements: {} vCPUs, {:.1} GB memory",
                required_vcpus,
                required_memory
            ));
        }

        // Score and sort instances based on strategy
        let scored_instances = suitable_instances
            .into_iter()
            .map(|(instance, count)| {
                let total_cost = instance.price_per_hour * count as f32;
                let total_vcpus = instance.vcpus * count;
                let total_memory = instance.memory_gb * count as f32;
                
                let cpu_utilization = requirements.total_vcpus as f32 / total_vcpus as f32;
                let memory_utilization = requirements.total_memory_gb / total_memory;
                let avg_utilization = (cpu_utilization + memory_utilization) / 2.0;

                let score = match strategy {
                    OptimizationStrategy::CostOptimized => {
                        // Lower cost is better, with utilization as secondary factor
                        1.0 / total_cost * avg_utilization
                    }
                    OptimizationStrategy::PerformanceOptimized => {
                        // Prefer newer generation and better network performance
                        let generation_score = if instance.name.contains("6i") { 1.2 } else { 1.0 };
                        let network_score = match instance.network_performance {
                            NetworkPerformance::VeryHigh => 1.2,
                            NetworkPerformance::TenGigabit => 1.3,
                            NetworkPerformance::TwentyFiveGigabit => 1.4,
                            _ => 1.0,
                        };
                        let family_score = match instance.family {
                            InstanceFamily::ComputeOptimized => 1.1,
                            InstanceFamily::MemoryOptimized => {
                                if requirements.total_memory_gb > requirements.total_vcpus as f32 * 4.0 {
                                    1.2 // Memory-heavy workload
                                } else {
                                    1.0
                                }
                            },
                            _ => 1.0,
                        };
                        generation_score * network_score * family_score / (count as f32).sqrt()
                    }
                    OptimizationStrategy::Balanced => {
                        // Balance between cost and performance
                        let cost_factor = 1.0 / total_cost.sqrt();
                        let utilization_factor = avg_utilization;
                        let generation_factor = if instance.name.contains("6i") { 1.1 } else { 1.0 };
                        cost_factor * utilization_factor * generation_factor
                    }
                };

                (instance, count, score, cpu_utilization, memory_utilization, total_cost)
            })
            .collect::<Vec<_>>();

        // Find best option
        let best = scored_instances
            .iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .context("Failed to find best instance")?;

        let (instance, count, _, cpu_util, mem_util, hourly_cost) = best;

        let recommendation_reason = match strategy {
            OptimizationStrategy::CostOptimized => {
                format!(
                    "Selected {} x {} for lowest cost while meeting requirements",
                    count, instance.name
                )
            }
            OptimizationStrategy::PerformanceOptimized => {
                format!(
                    "Selected {} x {} for best performance with {:?} network and {:?} family",
                    count, instance.name, instance.network_performance, instance.family
                )
            }
            OptimizationStrategy::Balanced => {
                format!(
                    "Selected {} x {} for balanced cost/performance ratio",
                    count, instance.name
                )
            }
        };

        Ok(InstanceRecommendation {
            instance_type: instance.clone(),
            instance_count: *count,
            total_vcpus: instance.vcpus * count,
            total_memory_gb: instance.memory_gb * (*count as f32),
            cpu_utilization: *cpu_util,
            memory_utilization: *mem_util,
            hourly_cost: *hourly_cost,
            monthly_cost: hourly_cost * 24.0 * 30.0,
            recommendation_reason,
        })
    }

    /// Calculate recommendations for multiple service groups
    pub fn calculate_multi_group_recommendations(
        &self,
        groups: Vec<ServiceGroupRequirements>,
        strategy: OptimizationStrategy,
    ) -> Result<Vec<(String, InstanceRecommendation)>> {
        let mut recommendations = Vec::new();

        for group in groups {
            let recommendation = self.calculate_recommendation(&group, strategy)?;
            recommendations.push((group.name.clone(), recommendation));
        }

        Ok(recommendations)
    }

    /// Print recommendation summary
    pub fn print_recommendation_summary(recommendation: &InstanceRecommendation) {
        info!("\n=== Instance Recommendation ===");
        info!("Instance Type: {}", recommendation.instance_type.name);
        info!("Instance Count: {}", recommendation.instance_count);
        info!("Total Resources: {} vCPUs, {:.1} GB RAM", 
            recommendation.total_vcpus, 
            recommendation.total_memory_gb
        );
        info!("Resource Utilization:");
        info!("  - CPU: {:.1}%", recommendation.cpu_utilization * 100.0);
        info!("  - Memory: {:.1}%", recommendation.memory_utilization * 100.0);
        info!("Estimated Cost:");
        info!("  - Hourly: ${:.2}", recommendation.hourly_cost);
        info!("  - Monthly: ${:.2}", recommendation.monthly_cost);
        info!("  - Yearly: ${:.2}", recommendation.monthly_cost * 12.0);
        info!("Reason: {}", recommendation.recommendation_reason);
    }

    /// Print summary report for all recommendations
    pub fn print_summary_report(recommendations: &[(String, InstanceRecommendation)]) {
        info!("\n=== EC2 Instance Summary Report ===");
        
        let mut total_instances = 0;
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut total_hourly_cost = 0.0;
        
        // Group by instance type
        let mut instance_type_counts: HashMap<String, u32> = HashMap::new();
        
        for (_, rec) in recommendations {
            total_instances += rec.instance_count;
            total_vcpus += rec.total_vcpus;
            total_memory_gb += rec.total_memory_gb;
            total_hourly_cost += rec.hourly_cost;
            
            *instance_type_counts.entry(rec.instance_type.name.clone()).or_insert(0) += rec.instance_count;
        }
        
        info!("\nTotal Resources Required:");
        info!("  - Instances: {}", total_instances);
        info!("  - vCPUs: {}", total_vcpus);
        info!("  - Memory: {:.1} GB", total_memory_gb);
        
        info!("\nInstance Type Distribution:");
        let mut types: Vec<_> = instance_type_counts.iter().collect();
        types.sort_by_key(|(name, _)| name.as_str());
        for (instance_type, count) in types {
            info!("  - {}: {} instances", instance_type, count);
        }
        
        info!("\nTotal Cost Estimation:");
        info!("  - Hourly: ${:.2}", total_hourly_cost);
        info!("  - Daily: ${:.2}", total_hourly_cost * 24.0);
        info!("  - Monthly: ${:.2}", total_hourly_cost * 24.0 * 30.0);
        info!("  - Yearly: ${:.2}", total_hourly_cost * 24.0 * 365.0);
        
        // Cost savings with reserved instances
        let one_year_reserved_discount = 0.28; // ~28% discount
        let three_year_reserved_discount = 0.48; // ~48% discount
        
        info!("\nPotential Savings with Reserved Instances:");
        info!("  - 1-Year Reserved: ${:.2}/year (save ${:.2})", 
            total_hourly_cost * 24.0 * 365.0 * (1.0 - one_year_reserved_discount),
            total_hourly_cost * 24.0 * 365.0 * one_year_reserved_discount
        );
        info!("  - 3-Year Reserved: ${:.2}/year (save ${:.2})", 
            total_hourly_cost * 24.0 * 365.0 * (1.0 - three_year_reserved_discount),
            total_hourly_cost * 24.0 * 365.0 * three_year_reserved_discount
        );
    }

    /// Calculate service group requirements from AWS service configs
    pub fn calculate_service_group_requirements(
        name: &str,
        services: Vec<(&str, u32, u32, u32)>, // (service_name, cpu, memory, task_count)
    ) -> ServiceGroupRequirements {
        let mut total_vcpus = 0;
        let mut total_memory_gb = 0.0;
        let mut instances_count = 0;

        for (_, cpu, memory, task_count) in &services {
            total_vcpus += cpu * task_count;
            total_memory_gb += (*memory as f32 / 1024.0) * *task_count as f32;
            instances_count += task_count;
        }

        // Determine if network intensive (edge services typically are)
        let network_intensive = services.iter().any(|(name, _, _, _)| name.contains("edge"));

        ServiceGroupRequirements {
            name: name.to_string(),
            total_vcpus,
            total_memory_gb,
            instances_count,
            network_intensive,
        }
    }

    /// Get recommended instance types for Redis cluster on EC2
    pub fn recommend_redis_cluster_instances(node_count: u32) -> InstanceRecommendation {
        // Redis typically needs:
        // - High memory for caching
        // - Good network performance for replication
        // - Moderate CPU (not compute intensive)
        
        // For production Redis, r6i.large (2 vCPU, 16GB RAM) is a good starting point
        let instance = InstanceType {
            name: "r6i.large".to_string(),
            family: InstanceFamily::MemoryOptimized,
            vcpus: 2,
            memory_gb: 16.0,
            price_per_hour: 0.126,
            network_performance: NetworkPerformance::High,
            ebs_optimized: true,
        };

        InstanceRecommendation {
            instance_type: instance.clone(),
            instance_count: node_count,
            total_vcpus: instance.vcpus * node_count,
            total_memory_gb: instance.memory_gb * node_count as f32,
            cpu_utilization: 0.5, // Redis is not CPU intensive
            memory_utilization: 0.7, // Good memory utilization
            hourly_cost: instance.price_per_hour * node_count as f32,
            monthly_cost: instance.price_per_hour * node_count as f32 * 24.0 * 30.0,
            recommendation_reason: format!(
                "Selected {} x {} for Redis cluster - memory optimized with good network performance",
                node_count, instance.name
            ),
        }
    }

    /// Get recommended instance types for ScyllaDB cluster on EC2
    pub fn recommend_scylladb_cluster_instances(
        node_count: u32,
        high_performance: bool,
    ) -> InstanceRecommendation {
        // ScyllaDB requirements:
        // - High CPU for processing
        // - High memory for caching
        // - Very high disk I/O
        // - Excellent network performance for replication
        
        let instance = if high_performance {
            // i3en.2xlarge: 8 vCPU, 64GB RAM, 2x2.5TB NVMe SSD
            InstanceType {
                name: "i3en.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.752,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            }
        } else {
            // r6i.2xlarge: 8 vCPU, 64GB RAM (needs EBS volumes)
            InstanceType {
                name: "r6i.2xlarge".to_string(),
                family: InstanceFamily::MemoryOptimized,
                vcpus: 8,
                memory_gb: 64.0,
                price_per_hour: 0.504,
                network_performance: NetworkPerformance::VeryHigh,
                ebs_optimized: true,
            }
        };

        InstanceRecommendation {
            instance_type: instance.clone(),
            instance_count: node_count,
            total_vcpus: instance.vcpus * node_count,
            total_memory_gb: instance.memory_gb * node_count as f32,
            cpu_utilization: 0.7,
            memory_utilization: 0.8,
            hourly_cost: instance.price_per_hour * node_count as f32,
            monthly_cost: instance.price_per_hour * node_count as f32 * 24.0 * 30.0,
            recommendation_reason: format!(
                "Selected {} x {} for ScyllaDB cluster - {} with excellent I/O and network",
                node_count,
                instance.name,
                if high_performance { "NVMe storage" } else { "EBS optimized" }
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_recommendation() {
        let calculator = InstanceCalculator::new();
        
        let requirements = ServiceGroupRequirements {
            name: "test-group".to_string(),
            total_vcpus: 16,
            total_memory_gb: 32.0,
            instances_count: 3,
            network_intensive: false,
        };

        let recommendation = calculator
            .calculate_recommendation(&requirements, OptimizationStrategy::Balanced)
            .unwrap();

        assert!(recommendation.total_vcpus >= 20); // With 25% margin
        assert!(recommendation.total_memory_gb >= 40.0); // With 25% margin
    }
}