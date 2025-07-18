use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::{
        hash_types::RichField,
        hashing::{hash_n_to_hash_no_pad, PlonkyPermutation},
    },
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_core::data::{
    qhashout::QHashOut,
    secp256k1::{bytes_to_u32_vec_le, CompressedPublicKey},
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub fn hash_no_pad_compressed_publicKey<F: RichField, P: PlonkyPermutation<F>>(
    secp256k1_public_key: CompressedPublicKey,
) -> QHashOut<F> {
    let mut secp256k1_public_key_bytes = vec![secp256k1_public_key.0[0], 0, 0, 0];
    secp256k1_public_key_bytes.extend_from_slice(&secp256k1_public_key.0[1..]);
    let secp256k1_public_key_f = bytes_to_u32_vec_le(&secp256k1_public_key_bytes)
        .iter()
        .map(|n| F::from_canonical_u32(*n))
        .collect::<Vec<_>>();

    QHashOut(hash_n_to_hash_no_pad::<F, P>(&secp256k1_public_key_f))
}
