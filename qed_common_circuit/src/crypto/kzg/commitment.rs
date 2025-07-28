use plonky2::{
    field::{
        extension::Extendable,
        types::{Field, PrimeField},
    },
    hash::hash_types::RichField,
    iop::target::Target,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
    },
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint;

#[derive(Clone, Debug)]
pub struct KZGCommitmentTarget<F: RichField + Extendable<D>, const D: usize> {
    pub commitment: G1AffineTarget<F, D>,
}

#[derive(Clone, Debug)]
pub struct KZGCommitment {
    pub commitment: AffinePoint<crate::crypto::bn254::curve::g1::G1>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kzg::CircuitBuilderKZG;
    use plonky2::{
        iop::witness::PartialWitness,
        plonk::{
            circuit_data::CircuitConfig,
            config::{GenericConfig, PoseidonGoldilocksConfig},
        },
    };

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn gen_test_config() -> CircuitConfig {
        CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        }
    }

    #[test]
    fn test_kzg_commitment_structure() {
        let config = gen_test_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        let coeff1 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
        let coeff2 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
        let coefficients = vec![coeff1, coeff2];
        
        let g1_gen = builder.g1_generator();
        let powers_of_tau = vec![g1_gen.clone(), g1_gen.clone()];
        
        let commitment = builder.kzg_commit(&coefficients, &powers_of_tau);
    }
}