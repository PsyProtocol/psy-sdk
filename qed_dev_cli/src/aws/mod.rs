pub mod instance_calculator;
pub mod simple_instance_selector;

pub use instance_calculator::{
    InstanceCalculator, 
    InstanceType, 
    InstanceFamily,
    OptimizationStrategy,
    ServiceGroupRequirements,
    InstanceRecommendation,
};

pub use simple_instance_selector::{
    SimpleInstanceSelector,
    SimpleInstance,
    SimpleInstanceRecommendation,
    ServiceType,
    ServiceGroupRequirements as SimpleServiceGroupRequirements,
};