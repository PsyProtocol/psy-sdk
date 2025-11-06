use anyhow::{anyhow, Result};
use async_trait::async_trait;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use psy_crypto::signature::zk::data::ZKPublicKeyInfo;
use psy_provider::provider::RpcProvider;
use psy_ups_circuit::signature::software_defined::get_sdc_public_key_param;
use psy_vm::ups::circuit_manager::UPSCircuitManager;

use crate::{
    signature::{
        context::SignContext,
        traits::{SignatureCircuitInfo, SignatureConfig, SignatureProof, SignatureUser, SIGNATURE_D},
    },
    wallet::memory_wallet::PsyMemoryWallet,
};

/// Software-Defined PLONKY2 signature user.
/// Handles signatures for custom PLONKY2-based software-defined circuits with
/// user-defined custom_sign logic.
#[derive(Debug, Clone)]
pub struct SoftwareDefinedPlonky2User {
    private_key: QHashOut<GoldilocksField>,
    fingerprint: QHashOut<GoldilocksField>,
}

impl SoftwareDefinedPlonky2User {
    pub fn new(private_key: QHashOut<GoldilocksField>, fingerprint: QHashOut<GoldilocksField>) -> Self {
        Self { private_key, fingerprint }
    }
}

#[async_trait]
impl SignatureUser for SoftwareDefinedPlonky2User {
    async fn public_key_info(
        &self,
        _wallet: &PsyMemoryWallet,
        _circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
    ) -> Result<ZKPublicKeyInfo<GoldilocksField>> {
        let public_key_param = get_sdc_public_key_param(&self.private_key);
        Ok(ZKPublicKeyInfo {
            fingerprint: self.fingerprint,
            public_key_param,
        })
    }

    async fn sign(
        &self,
        wallet: &PsyMemoryWallet,
        _circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        context: &SignContext,
        sighash: QHashOut<GoldilocksField>,
    ) -> Result<SignatureProof> {
        let plonky2_input = context
            .plonky2_signature_input
            .as_ref()
            .ok_or_else(|| anyhow!("PLONKY2 signature input missing for PLONKY2 user"))?;

        if context.psy_witness_input.is_some() {
            return Err(anyhow!("SoftwareDefinedPlonky2User cannot handle PSY witness input"));
        }

        let mut circuit = wallet
            .get_plonky2_software_defined_circuit_mut(&self.fingerprint)
            .ok_or_else(|| anyhow!("PLONKY2 software defined circuit `{}` not registered", self.fingerprint))?;

        circuit.prove(self.private_key, plonky2_input, sighash).await
    }

    async fn circuit_info(
        &self,
        wallet: &PsyMemoryWallet,
        _circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        context: &SignContext,
    ) -> Result<SignatureCircuitInfo> {
        if context.plonky2_signature_input.is_none() {
            return Err(anyhow!("PLONKY2 signature input missing for PLONKY2 user"));
        }

        if context.psy_witness_input.is_some() {
            return Err(anyhow!("SoftwareDefinedPlonky2User cannot handle PSY witness input"));
        }

        let circuit = wallet
            .get_plonky2_software_defined_circuit(&self.fingerprint)
            .ok_or_else(|| anyhow!("PLONKY2 software defined circuit `{}` not registered", self.fingerprint))?;

        Ok(SignatureCircuitInfo {
            circuit_fingerprint: circuit.get_fingerprint(),
            verifier_config: circuit
                .get_verifier_config_ref()
                .ok_or_else(|| anyhow!("Verifier config not available"))?
                .clone(),
        })
    }
}
