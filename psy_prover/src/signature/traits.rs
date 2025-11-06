use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{circuit_data::VerifierOnlyCircuitData, config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::{hash::traits::qhashable::QFieldHashable, signature::zk::data::ZKPublicKeyInfo};
use psy_data::config::store_config::PsyHasher;
use psy_provider::provider::RpcProvider;
use psy_vm::ups::circuit_manager::UPSCircuitManager;

use super::context::SignContext;
use crate::wallet::memory_wallet::PsyMemoryWallet;

pub const SIGNATURE_D: usize = 2;
pub type SignatureConfig = PoseidonGoldilocksConfig;
pub type SignatureProof = ProofWithPublicInputs<GoldilocksField, SignatureConfig, SIGNATURE_D>;

pub struct SignatureCircuitInfo {
    pub circuit_fingerprint: QHashOut<GoldilocksField>,
    pub verifier_config: VerifierOnlyCircuitData<SignatureConfig, SIGNATURE_D>,
}

impl fmt::Debug for SignatureCircuitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignatureCircuitInfo")
            .field("circuit_fingerprint", &self.circuit_fingerprint)
            .field("verifier_config", &"...")
            .finish()
    }
}

pub struct SignatureResult {
    pub proof: SignatureProof,
    pub circuit_info: SignatureCircuitInfo,
}

impl fmt::Debug for SignatureResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignatureResult")
            .field("proof_public_inputs", &self.proof.public_inputs)
            .field("circuit_info", &self.circuit_info)
            .finish()
    }
}

#[async_trait]
pub trait SignatureUser: fmt::Debug + Send + Sync {
    async fn public_key_info(
        &self,
        wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
    ) -> Result<ZKPublicKeyInfo<GoldilocksField>>;

    async fn sign(
        &self,
        wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        context: &SignContext,
        sighash: QHashOut<GoldilocksField>,
    ) -> Result<SignatureProof>;

    async fn circuit_info(
        &self,
        _wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        _context: &SignContext,
    ) -> Result<SignatureCircuitInfo>;

    async fn public_key_hash(
        &self,
        wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
    ) -> Result<QHashOut<GoldilocksField>> {
        let info = self.public_key_info(wallet, circuit_manager).await?;
        Ok(info.qfhash::<PsyHasher>())
    }
}
