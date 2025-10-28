use anyhow::Ok;
use dashmap::DashMap;
use k256::ecdsa::signature::hazmat::PrehashSigner;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::{
        hash_types::HashOut,
        poseidon::{PoseidonHash, PoseidonPermutation},
    },
    plonk::{
        config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common_circuit::circuits::{
    secp256k1_signature::Secp256K1SignatureCircuit, traits::qstandard::QStandardCircuit, zk_signature3::core::PsyBasicZKSignatureCircuit,
};
use psy_core::{
    config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::{base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::CompressedPublicKey},
};
use psy_crypto::{
    hash::traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable},
    signature::{
        secp256k1::core::PsyCompressedSecp256K1Signature,
        zk::{data::ZKPublicKeyInfo, wallet::SimplePsyPrivateKey},
    },
};
use psy_data::{config::store_config::PsyHasher, qstore::imm::cmd_processor::PsyReadCommandProcessorSync};
use psy_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
use psy_rust_sdk::provider::UPSCircuitManagerTrait;

use psy_ups_circuit::{circuit_manager, circuit_manager::core::QCircuitManager};
use crate::{
    local::args::SignType,
    wallet::{
        simple_sign::StateReader,
        software_defined_circuit::{
            get_sdc_public_key_param, QSoftwareDefinedSignatureGadget, SoftwareDefinedSignature, SoftwareDefinedSignatureCircuit,
            SoftwareDefinedSignatureGadget, SoftwareDefinedSignatureInput,
        },
    },
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

// #[derive(Clone)]
pub struct PsyMemoryWallet {
    // pub zk_circuit: Option<PsyBasicZKSignatureCircuit<C, D>>,
    // pub secp_circuit: Option<Secp256K1SignatureCircuit<C, D>>,
    // figerprint, circuit
    pub software_defined_circuits: DashMap<QHashOut<F>, SoftwareDefinedSignatureCircuit<C, D, SoftwareDefinedSignatureGadget>>,
    pub circuit_manager: Vec<Box<dyn UPSCircuitManagerTrait<C, D> + Send + Sync>>,
    pub zk_public_key_to_private_key_store: DashMap<QHashOut<F>, QHashOut<F>>,
    pub secp_public_key_to_private_key_store: DashMap<QHashOut<F>, QHashOut<F>>,
    pub software_defined_public_key_to_private_key_store: DashMap<QHashOut<F>, QHashOut<F>>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl PsyMemoryWallet {
    pub fn new(circuit_manager: Vec<Box<dyn UPSCircuitManagerTrait<C, D> + Send + Sync>>) -> Self {
        Self {
            zk_public_key_to_private_key_store: DashMap::new(),
            secp_public_key_to_private_key_store: DashMap::new(),
            software_defined_public_key_to_private_key_store: DashMap::new(),
            circuit_manager,
            software_defined_circuits: DashMap::new(),
        }
    }

    pub fn random_circuit_manager(&self) -> &Box<dyn UPSCircuitManagerTrait<C, D> + Send + Sync> {
        let index = rand::random::<usize>() % self.circuit_manager.len();
        &self.circuit_manager[index]
    }

    pub async fn add_zk_private_key(&mut self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let pk_info = self.get_zk_pk_info(private_key).await?;
        self.zk_public_key_to_private_key_store.insert(pk_info.qfhash::<PsyHasher>(), private_key);
        Ok(pk_info)
    }

    pub async fn get_zk_pk_info(&self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let private_key = SimplePsyPrivateKey { private_key };
        let public_key_param = private_key.get_public_key_param::<PoseidonHash>();
        let fingerprint = self.random_circuit_manager().zk_circuit_fingerprint().await?;

        Ok(ZKPublicKeyInfo {
            fingerprint,
            public_key_param,
        })
    }

    pub async fn get_sign_type(&self, pk_hash: QHashOut<F>) -> anyhow::Result<SignType> {
        if self.zk_public_key_to_private_key_store.contains_key(&pk_hash) {
            Ok(SignType::ZKSign)
        } else if self.secp_public_key_to_private_key_store.contains_key(&pk_hash) {
            Ok(SignType::SECP256K1Sign)
        } else if self.software_defined_public_key_to_private_key_store.contains_key(&pk_hash) {
            Ok(SignType::SoftwareDefinedSign)
        } else {
            Err(anyhow::format_err!("pk_hash `{}` not found", pk_hash))
        }
    }

    pub async fn add_secp_private_key(&mut self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let pk_info = self.get_secp_pk_info(private_key).await?;
        tracing::info!("add secp user {}", serde_json::to_string_pretty(&pk_info)?);

        self.secp_public_key_to_private_key_store
            .insert(pk_info.qfhash::<PsyHasher>(), private_key);
        Ok(pk_info)
    }

    pub async fn add_software_defined_private_key(
        &mut self,
        private_key: QHashOut<F>,
        fingerprint: QHashOut<F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let public_key_param = get_sdc_public_key_param(&private_key);
        let pk_info = ZKPublicKeyInfo {
            fingerprint: fingerprint,
            public_key_param,
        };
        tracing::info!("add software defined user {}", serde_json::to_string_pretty(&pk_info)?);

        self.software_defined_public_key_to_private_key_store
            .insert(pk_info.qfhash::<PsyHasher>(), private_key);
        Ok(pk_info)
    }

    pub fn get_secp_public_key(&self, private_key: QHashOut<F>) -> anyhow::Result<CompressedPublicKey> {
        psy_crypto::signature::secp256k1::wallet::get_secp_public_key(private_key)
    }

    pub async fn get_secp_pk_info(&self, private_key: QHashOut<F>) -> anyhow::Result<ZKPublicKeyInfo<F>> {
        let pub_compressed = self.get_secp_public_key(private_key)?;
        tracing::info!("get secp public key {:?}", pub_compressed);

        let public_key_params = psy_crypto::signature::secp256k1::wallet::hash_no_pad_compressed_public_key::<F, PoseidonPermutation<F>>(pub_compressed);

        Ok(ZKPublicKeyInfo {
            fingerprint: self.random_circuit_manager().secp_circuit_fingerprint().await?,
            public_key_param: public_key_params,
        })
    }

    pub async fn zk_sign_for_public_key(&self, public_key: QHashOut<F>, sig_hash: QHashOut<F>) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        let private_key = self.zk_public_key_to_private_key_store.get(&public_key).ok_or(anyhow::format_err!(
            "tried to sign with a public key ({}) which does not match any private keys in the store",
            public_key.to_string()
        ))?;
        self.random_circuit_manager().prove_zk_sign(*private_key, sig_hash).await
    }

    pub async fn zk_sign_with_private_key(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        self.random_circuit_manager().prove_zk_sign(private_key, sig_hash).await
    }

    pub fn sdc_sign_for_public_key<R: PsyReadCommandProcessorSync<F> + Send + Sync>(
        &self,
        state_reader: &mut StateReader<F, D, R>,
        public_key: QHashOut<F>,
        sig_hash: QHashOut<F>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        unimplemented!()
    }

    pub fn sdc_sign_with_private_key<R: PsyReadCommandProcessorSync<F> + Send + Sync>(
        &self,
        state_reader: &mut StateReader<F, D, R>,
        private_key: QHashOut<F>,
        sig_hash: QHashOut<F>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        unimplemented!()
    }

    pub fn secp256k1_sign(&self, private_key: QHashOut<F>, sig_hash: QHashOut<F>) -> anyhow::Result<PsyCompressedSecp256K1Signature> {
        let signing_key = k256::ecdsa::SigningKey::from_slice(&Hash256::from(private_key).0)?;
        let result: k256::ecdsa::Signature = signing_key.sign_prehash(&Hash256::from(sig_hash).0)?;
        let mut rs_bytes = [0u8; 64];

        let r_bytes = result.r().to_bytes();
        let s_bytes = result.s().to_bytes();
        rs_bytes[0..32].copy_from_slice(&r_bytes);
        rs_bytes[32..64].copy_from_slice(&s_bytes);

        Ok(PsyCompressedSecp256K1Signature {
            public_key: self.get_secp_public_key(private_key)?.0,
            signature: rs_bytes,
            message: Hash256::from(sig_hash),
        })
    }

    pub fn secp256k1_sign_with_public_key(&self, public_key: QHashOut<F>, sig_hash: QHashOut<F>) -> anyhow::Result<PsyCompressedSecp256K1Signature> {
        let private_key = self.secp_public_key_to_private_key_store.get(&public_key).ok_or(anyhow::format_err!(
            "public key ({}) does not match any private keys",
            public_key.to_string()
        ))?;
        self.secp256k1_sign(*private_key, sig_hash)
    }

    pub async fn zk_secp256k1_from_signature(&self, signature: &PsyCompressedSecp256K1Signature) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        self.random_circuit_manager().prove_secp_sign(*signature).await
    }

    pub async fn zk_sign_secp256k1(&self, public_key: QHashOut<F>, sig_hash: QHashOut<F>) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        let ecc_sig = self.secp256k1_sign_with_public_key(public_key, sig_hash)?;
        self.random_circuit_manager().prove_secp_sign(ecc_sig).await
    }
}

/// software defined circuit
#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl PsyMemoryWallet {
    pub async fn register_software_defined_circuit(&mut self, input: SoftwareDefinedSignatureInput) -> anyhow::Result<QHashOut<F>> {
        // Convert prover's enum to SDK's enum
        let sdk_input = match input {
            SoftwareDefinedSignatureInput::Psy(psy_input) => {
                psy_rust_sdk::wallet::software_defined_circuit::SoftwareDefinedSignatureInput::Psy(psy_input)
            }
            SoftwareDefinedSignatureInput::PLONKY2(_) => {
                anyhow::bail!("PLONKY2 variant not supported for SDK integration")
            }
        };
        self.random_circuit_manager().register_software_defined_circuit(sdk_input).await
        // if let QCircuitManager::Rpc(rpc_provider) =
        // self.random_circuit_manager() {     return
        // rpc_provider.register_software_defined_circuit(input).await;
        // };
        // let sdc = SoftwareDefinedSignatureCircuit::new(&input).await;
        // let fingerprint = sdc.get_fingerprint();
        // tracing::info!(
        //     "register software defined circuit: {}",
        //     fingerprint.to_string()
        // );
        // if let Some(_) = self.software_defined_circuits.insert(fingerprint,
        // sdc) {     tracing::warn!(
        //         "software defined circuit `{}` is already registered",
        //         fingerprint.to_string()
        //     );
        // };
        // Ok(fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::Result;
    use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
    use psy_common_circuit::circuits::{secp256k1_signature::Secp256K1SignatureCircuit, traits::qstandard::QStandardCircuit};
    use psy_core::data::qhashout::QHashOut;

    use super::*;

    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    #[test]
    fn test_raw_secp256k1_sign() -> Result<()> {
        use k256::ecdsa::signature::hazmat::PrehashSigner;
        use psy_core::data::base_types::hash256::Hash256;
        use psy_crypto::signature::secp256k1::core::PsyCompressedSecp256K1Signature;

        // Create a test private key and signature hash
        let private_key = QHashOut::<F>::from_str("17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a")?;
        let sig_hash = QHashOut::<F>::from_str("83955402ec7f375d1d6e8f3bf59753fe0af1e7c62bb4b662716a2524d3e2d186")?;

        // Test signature generation with reverse (like in memory_wallet)
        let signing_key = k256::ecdsa::SigningKey::from_slice(&Hash256::from(private_key).0)?;
        let mut sig_hash_bytes = Hash256::from(sig_hash).0;
        let result: k256::ecdsa::Signature = signing_key.sign_prehash(&sig_hash_bytes)?;

        let mut rs_bytes = [0u8; 64];
        let r_bytes = result.r().to_bytes();
        let s_bytes = result.s().to_bytes();
        rs_bytes[0..32].copy_from_slice(&r_bytes);
        rs_bytes[32..64].copy_from_slice(&s_bytes);

        // Get compressed public key
        let pk = signing_key.verifying_key();
        let pk_bytes = pk.to_encoded_point(true).to_bytes();
        let mut compressed_pk = [0u8; 33];
        compressed_pk.copy_from_slice(&pk_bytes);

        let secp_signature = PsyCompressedSecp256K1Signature {
            public_key: compressed_pk,
            signature: rs_bytes,
            message: Hash256::from(sig_hash),
        };

        println!("Generated signature with reverse:");
        println!("  Public key: {:?}", hex::encode(&secp_signature.public_key));
        println!("  Signature: {:?}", hex::encode(&secp_signature.signature));
        println!("  Message: {:?}", hex::encode(&secp_signature.message.0));

        // Create SECP256K1 signature circuit and test
        let secp_circuit = Secp256K1SignatureCircuit::<C, D>::new();

        println!("Created SECP256K1 circuit, fingerprint: {}", secp_circuit.get_fingerprint());

        // Generate ZK proof using the circuit
        let zk_proof = secp_circuit.prove(&secp_signature)?;

        println!("Generated ZK proof with {} public inputs", zk_proof.public_inputs.len());
        println!("Public inputs: {:?}", zk_proof.public_inputs);

        // Verify the public inputs match expected format: hash(sighash,
        // public_key_param)
        let combined_hash_from_proof = QHashOut(plonky2::hash::hash_types::HashOut {
            elements: [
                zk_proof.public_inputs[0],
                zk_proof.public_inputs[1],
                zk_proof.public_inputs[2],
                zk_proof.public_inputs[3],
            ],
        });

        println!("Circuit public inputs (combined hash): {}", combined_hash_from_proof);

        // Calculate expected combined hash: hash(sighash, public_key_param)
        use plonky2::hash::poseidon::PoseidonPermutation;
        use psy_crypto::hash::traits::hasher::FieldQHasher;
        use psy_data::config::store_config::PsyHasher;

        let public_key_param = psy_crypto::signature::secp256k1::wallet::hash_no_pad_compressed_public_key::<F, PoseidonPermutation<F>>(
            psy_core::data::secp256k1::CompressedPublicKey(compressed_pk),
        );
        let message_hash: QHashOut<F> = QHashOut::from(Hash256::from(sig_hash));

        let expected_combined_hash = PsyHasher::q_two_to_one(message_hash, public_key_param);

        println!(
            "Expected combined hash: hash({}, {}) = {}",
            message_hash, public_key_param, expected_combined_hash
        );

        assert_eq!(
            combined_hash_from_proof, expected_combined_hash,
            "Raw secp256k1 proof public inputs should match hash(sighash, public_key_param)"
        );

        Ok(())
    }

    #[test]
    fn test_memory_wallet_secp256k1_sign() -> Result<()> {
        use psy_core::data::base_types::hash256::Hash256;
        use psy_crypto::signature::secp256k1::core::PsyCompressedSecp256K1Signature;

        // Create a test private key and signature hash
        let private_key = QHashOut::<F>::from_str("17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a")?;
        let sig_hash = QHashOut::<F>::from_str("83955402ec7f375d1d6e8f3bf59753fe0af1e7c62bb4b662716a2524d3e2d186")?;

        // Create a mock memory wallet for testing
        let circuit_manager = psy_ups_circuit::circuit_manager::core::QCircuitManager::Local(
            psy_ups_circuit::circuit_manager::core::PsyUPSStepCircuitManager::new_with_config(0x1337),
        );
        let wallet = PsyMemoryWallet::new(vec![Box::new(circuit_manager)]);

        println!("Created memory wallet");

        // Generate SECP256K1 signature using memory wallet method
        let secp_signature = wallet.secp256k1_sign(private_key, sig_hash)?;

        println!("Generated signature using memory wallet:");
        println!("  Public key: {:?}", hex::encode(&secp_signature.public_key));
        println!("  Signature: {:?}", hex::encode(&secp_signature.signature));
        println!("  Message: {:?}", hex::encode(&secp_signature.message.0));

        // Create SECP256K1 signature circuit and test
        let secp_circuit = Secp256K1SignatureCircuit::<C, D>::new();

        println!("Created SECP256K1 circuit, fingerprint: {}", secp_circuit.get_fingerprint());

        // Generate ZK proof using the circuit
        let zk_proof = secp_circuit.prove(&secp_signature)?;

        println!("Generated ZK proof with {} public inputs", zk_proof.public_inputs.len());
        println!("Public inputs: {:?}", zk_proof.public_inputs);

        // ZK proof generated successfully (verification may have circuit structure
        // issues)
        println!("✅ ZK proof generation succeeded!");

        // The public inputs should be the combined hash of sighash and public key
        let combined_hash_from_proof = QHashOut(plonky2::hash::hash_types::HashOut {
            elements: [
                zk_proof.public_inputs[0],
                zk_proof.public_inputs[1],
                zk_proof.public_inputs[2],
                zk_proof.public_inputs[3],
            ],
        });

        println!("Circuit combined hash output: {}", combined_hash_from_proof);

        // Verify this matches expected format: hash(message_hash, public_key_param)
        use plonky2::hash::poseidon::PoseidonPermutation;
        use psy_crypto::hash::traits::hasher::FieldQHasher;
        use psy_data::config::store_config::PsyHasher;

        // Get public key param the same way as in memory wallet
        let public_key_param = psy_crypto::signature::secp256k1::wallet::hash_no_pad_compressed_public_key::<F, PoseidonPermutation<F>>(
            psy_core::data::secp256k1::CompressedPublicKey(secp_signature.public_key),
        );
        let message_hash: QHashOut<F> = QHashOut::from(secp_signature.message);

        let expected_combined_hash = PsyHasher::q_two_to_one(message_hash, public_key_param);

        println!(
            "Expected combined hash: hash({}, {}) = {}",
            message_hash, public_key_param, expected_combined_hash
        );

        assert_eq!(
            combined_hash_from_proof, expected_combined_hash,
            "Proof public inputs should match hash(sighash, public_key_hash)"
        );

        Ok(())
    }
}
