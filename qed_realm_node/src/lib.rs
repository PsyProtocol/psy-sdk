mod processor;

pub use processor::*;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_store::config::store_config::QEDFelt;
