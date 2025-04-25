use qed_store::config::store_config::QEDHasher;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_store::config::store_config::QEDFelt;

pub type H = QEDHasher;

mod config;
mod edge;
mod processor;

pub use config::*;
pub use edge::*;
pub use processor::*;
