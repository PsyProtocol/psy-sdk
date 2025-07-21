use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};
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
use qed_crypto::signature::secp256k1::core::QEDCompressedSecp256K1Signature;

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

pub fn secp256k1_sign<F: RichField>(
    private_key: SigningKey,
    sighash: QHashOut<F>,
) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
    tracing::info!("🔔 prove_secp256k1_signature");

    // let private_key: Hash256 = private_key.into();
    // let private_key = k256::ecdsa::SigningKey::from_slice(&private_key.0)?;
    let public_key = private_key
        .verifying_key()
        .to_encoded_point(true)
        .to_bytes();
    let mut compressed = [0u8; 33];
    if public_key.len() == 33 {
        compressed.copy_from_slice(&public_key);
    } else {
        return Err(anyhow::format_err!("pub key length is not 33"));
    }
    let pub_compressed = CompressedPublicKey(compressed);
    let result: k256::ecdsa::Signature = private_key.sign_prehash(&sighash.to_le_bytes())?;
    let mut rs_bytes = [0u8; 64];

    let r_bytes = result.r().to_bytes();
    let s_bytes = result.s().to_bytes();
    rs_bytes[0..32].copy_from_slice(&r_bytes);
    rs_bytes[32..64].copy_from_slice(&s_bytes);

    Ok(QEDCompressedSecp256K1Signature {
        public_key: pub_compressed.0,
        signature: rs_bytes,
        message: sighash.into(),
    })
}
