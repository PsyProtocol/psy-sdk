use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
pub mod api_request_id;
pub mod jobs;
pub mod traits;
pub mod verifier;
const D: usize = 2;
pub type ConcreteProofWithPublicInputs =
    ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, D>;
