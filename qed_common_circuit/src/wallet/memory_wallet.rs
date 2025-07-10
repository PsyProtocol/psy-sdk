use plonky2::plonk::{
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputs,
};
use qed_core::data::{
    base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::CompressedPublicKey,
};
use qed_crypto::signature::{
    secp256k1::{
        core::QEDCompressedSecp256K1Signature,
        wallet::{MemorySecp256K1Wallet, Secp256K1WalletProvider},
    },
    zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey},
};

use crate::circuits::{
    l1_secp256k1_signature::L1Secp256K1SignatureCircuit,
    zk_signature3::manager::SimpleQEDZKSignatureManager,
};

#[derive(Clone)]
pub struct QEDMemoryWallet<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub zk_wallet: SimpleQEDZKSignatureManager<C, D>,
    pub secp256k1_circuit: Option<L1Secp256K1SignatureCircuit<C, D>>,
    pub secp256k1_wallet: MemorySecp256K1Wallet,
}

impl<C: GenericConfig<D> + 'static, const D: usize> QEDMemoryWallet<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        Self {
            zk_wallet: SimpleQEDZKSignatureManager::<C, D>::new(),
            secp256k1_wallet: MemorySecp256K1Wallet::new(),
            secp256k1_circuit: Some(L1Secp256K1SignatureCircuit::new()),
        }
    }
    pub fn new_fast_setup() -> Self {
        Self {
            zk_wallet: SimpleQEDZKSignatureManager::<C, D>::new(),
            secp256k1_wallet: MemorySecp256K1Wallet::new(),
            secp256k1_circuit: None,
        }
    }
    pub fn setup_circuits(&mut self) {
        if self.secp256k1_circuit.is_none() {
            self.secp256k1_circuit = Some(L1Secp256K1SignatureCircuit::new());
        }
    }
    pub fn add_private_key(&mut self, private_key: QHashOut<C::F>) -> QHashOut<C::F> {
        self.zk_wallet
            .add_private_key(SimpleQEDPrivateKey { private_key })
    }
    pub fn get_public_key_info(&self, private_key: QHashOut<C::F>) -> ZKPublicKeyInfo<C::F> {
        self.zk_wallet
            .get_public_key_info(SimpleQEDPrivateKey { private_key })
    }
    pub fn add_private_key_get_info(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> ZKPublicKeyInfo<C::F> {
        self.zk_wallet
            .add_private_key_get_info(SimpleQEDPrivateKey { private_key })
    }
    pub fn add_secp256k1_private_key(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<CompressedPublicKey> {
        self.secp256k1_wallet.add_private_key(private_key.into())
    }
    pub fn zk_sign_for_public_key(
        &self,
        public_key: QHashOut<C::F>,
        message: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.zk_wallet.zk_sign_for_public_key(public_key, message)
    }
    pub fn zk_sign_with_private_key(
        &self,
        private_key: QHashOut<C::F>,
        message: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.zk_wallet
            .zk_sign_for_private_key_value(private_key, message)
    }

    pub fn sign_secp256k1(
        &self,
        public_key: CompressedPublicKey,
        message: Hash256,
    ) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
        self.secp256k1_wallet.sign(&public_key, message)
    }
    pub fn sign_hash_secp256k1(
        &self,
        public_key: CompressedPublicKey,
        message: QHashOut<C::F>,
    ) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
        self.secp256k1_wallet.sign_qhashout(&public_key, message)
    }
    pub fn zk_secp256k1_from_signature(
        &self,
        signature: &QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.secp256k1_circuit
            .as_ref()
            .ok_or(anyhow::anyhow!("secp256k1 circuit not setup"))?
            .prove(signature)
    }
    pub fn zk_sign_secp256k1(
        &self,
        public_key: CompressedPublicKey,
        message: Hash256,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let ecc_sig = self.sign_secp256k1(public_key, message)?;
        self.secp256k1_circuit
            .as_ref()
            .ok_or(anyhow::anyhow!("secp256k1 circuit not setup"))?
            .prove(&ecc_sig)
    }
    pub fn zk_sign_hash_secp256k1(
        &self,
        public_key: CompressedPublicKey,
        message: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let ecc_sig = self.sign_hash_secp256k1(public_key, message)?;
        self.secp256k1_circuit
            .as_ref()
            .ok_or(anyhow::anyhow!("secp256k1 circuit not setup"))?
            .prove(&ecc_sig)
    }
}
