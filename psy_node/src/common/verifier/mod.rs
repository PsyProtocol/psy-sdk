use cached_common_data::get_cached_common_data_library;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use psy_crypto::common::{cached_circuit_library::get_cached_circuit_library, generic_circuit_verifier::GenericCircuitVerifier};

pub mod cached_common_data;

pub fn get_cached_generic_verifier<C: GenericConfig<D>, const D: usize>() -> GenericCircuitVerifier<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    GenericCircuitVerifier {
        library: get_cached_circuit_library(),
        common: get_cached_common_data_library(),
    }
}
