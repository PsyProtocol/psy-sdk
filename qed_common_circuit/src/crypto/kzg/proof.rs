use plonky2::{
    field::{
        extension::Extendable,
        types::{Field, PrimeField},
    },
    hash::hash_types::RichField,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    field::bn128_scalar::Bn128Scalar,
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint;

#[derive(Clone, Debug)]
pub struct KZGProofTarget<F: RichField + Extendable<D>, const D: usize> {
    pub w: G1AffineTarget<F, D>,
}

#[derive(Clone, Debug)]
pub struct KZGProof {
    pub w: AffinePoint<crate::crypto::bn254::curve::g1::G1>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kzg::CircuitBuilderKZG;
    use crate::crypto::bn254::gadgets::g1::CircuitBuilderG1;
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
            num_wires: 500,
            ..CircuitConfig::wide_ecc_config()
        }
    }

    #[test]
    fn test_polynomial_evaluation() {
    }
}