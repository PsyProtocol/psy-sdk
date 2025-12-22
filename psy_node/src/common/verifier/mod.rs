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

mod tests {
    use plonky2::{
        field::goldilocks_field::GoldilocksField,
        plonk::{circuit_data::VerifierOnlyCircuitData, config::PoseidonGoldilocksConfig},
    };
    use psy_common::{data::alt::AltVerifierOnlyCircuitData, job::id::ProvingJobCircuitType};
    use psy_crypto::common::{
        cached_circuit_library::get_cached_circuit_library,
        circuit_library::{CircuitInfoLibrary, CircuitInfoLibraryCore},
    };

    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    type F = GoldilocksField;

    #[test]
    fn test_get_cached_generic_verifier() -> anyhow::Result<()> {
        let circuit_library = get_cached_circuit_library::<F>();
        let endcap_data: VerifierOnlyCircuitData<C, D> = circuit_library.get_verifier_data(ProvingJobCircuitType::UserEndCap)?;
        let endcap_fingerprint = circuit_library.get_fingerprint(ProvingJobCircuitType::UserEndCap)?;

        let alt_endcap_data = AltVerifierOnlyCircuitData::from(endcap_data);

        println!("endcap_data: {}", serde_json::to_string_pretty(&alt_endcap_data)?);
        println!("endcap_data: {}", serde_json::to_string(&alt_endcap_data)?);
        println!("endcap_fingerprint: {:?}", endcap_fingerprint);
        println!("endcap_fingerprint: {}", serde_json::to_string(&endcap_fingerprint)?);

        Ok(())
    }
}
