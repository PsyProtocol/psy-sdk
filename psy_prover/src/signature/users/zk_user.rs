use anyhow::Result;
use async_trait::async_trait;
use plonky2::{field::goldilocks_field::GoldilocksField, hash::poseidon::PoseidonHash};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::signature::zk::{data::ZKPublicKeyInfo, wallet::SimplePsyPrivateKey};
use psy_provider::provider::RpcProvider;
use psy_vm::ups::circuit_manager::UPSCircuitManager;

use crate::{
    signature::{
        context::SignContext,
        traits::{SignatureCircuitInfo, SignatureConfig, SignatureProof, SignatureUser, SIGNATURE_D},
    },
    wallet::memory_wallet::PsyMemoryWallet,
};

#[derive(Debug, Clone)]
pub struct ZKUser {
    private_key: SimplePsyPrivateKey<GoldilocksField>,
}

impl ZKUser {
    pub fn new(private_key: SimplePsyPrivateKey<GoldilocksField>) -> Self {
        Self { private_key }
    }
}

#[async_trait]
impl SignatureUser for ZKUser {
    async fn public_key_info(
        &self,
        _wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
    ) -> Result<ZKPublicKeyInfo<GoldilocksField>> {
        let fingerprint = circuit_manager.zk_circuit_fingerprint().await?;
        let public_key_param = self.private_key.get_public_key_param::<PoseidonHash>();
        Ok(ZKPublicKeyInfo {
            fingerprint,
            public_key_param,
        })
    }

    async fn sign(
        &self,
        _wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        _context: &SignContext,
        sighash: QHashOut<GoldilocksField>,
    ) -> Result<SignatureProof> {
        circuit_manager.prove_zk_sign(self.private_key.private_key, sighash).await
    }

    async fn circuit_info(
        &self,
        _wallet: &PsyMemoryWallet,
        circuit_manager: &(dyn UPSCircuitManager<SignatureConfig, SIGNATURE_D> + Send + Sync),
        _context: &SignContext,
    ) -> Result<SignatureCircuitInfo> {
        Ok(SignatureCircuitInfo {
            circuit_fingerprint: circuit_manager.zk_circuit_fingerprint().await?,
            verifier_config: circuit_manager.zk_circuit_verifier_config().await?,
        })
    }
}
