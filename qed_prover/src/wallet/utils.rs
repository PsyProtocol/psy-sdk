use plonky2::hash::{
    hash_types::RichField,
    hashing::{hash_n_to_hash_no_pad, PlonkyPermutation},
};
use qed_core::data::{
    base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::{bytes_to_u32_vec_le, CompressedPublicKey}
};

pub fn hash_no_pad_compressed_public_key<F: RichField, P: PlonkyPermutation<F>>(
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

pub fn get_secp_public_key<F: RichField>(
    private_key: QHashOut<F>,
) -> anyhow::Result<CompressedPublicKey> {
    let private_key = k256::ecdsa::SigningKey::from_slice(&Hash256::from(private_key).0)?;
    let public_key = private_key
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes();
    let mut compressed = [0u8; 33];
    if public_key.len() == 33 {
        compressed.copy_from_slice(&public_key);
    } else {
        anyhow::bail!("public key length is not 33")
    }
    Ok(CompressedPublicKey(compressed))
}

