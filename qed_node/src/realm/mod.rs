use qed_data::config::store_config::QEDHasher;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_data::config::store_config::QEDFelt;

pub type H = QEDHasher;

pub mod config;
pub mod edge;
pub mod processor;
pub mod state;

pub use config::*;
pub use edge::*;
pub use processor::*;
