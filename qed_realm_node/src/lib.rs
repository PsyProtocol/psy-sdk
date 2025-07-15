use qed_data::config::store_config::QEDHasher;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_data::config::store_config::QEDFelt;

pub type H = QEDHasher;

mod config;
mod edge;
mod processor;
mod queue;

pub use config::*;
pub use edge::*;
pub use processor::*;
pub use queue::*;