use anyhow::Ok;
use dashmap::DashMap;
use k256::ecdsa::signature::hazmat::PrehashSigner;
use plonky2::hash::hash_types::HashOut;
use plonky2::hash::poseidon::PoseidonPermutation;
use plonky2::plonk::{
    config::{AlgebraicHasher, GenericConfig},
    proof::ProofWithPublicInputs,
};
use qed_core::config::network_constants::{
    MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT,
};
use qed_core::data::{
    base_types::hash256::Hash256, qhashout::QHashOut, secp256k1::CompressedPublicKey,
};
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_crypto::signature::{
    secp256k1::core::QEDCompressedSecp256K1Signature,
    zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey},
};
use qed_data::qdata::user_contract_state::UserContractState;
use qed_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::wallet::simple_sign::{
    SimpleSoftwareDefinedCircuit, SoftwareDefinedSignGadget, SoftwareDefinedSignTrait,
};
use crate::wallet::software_defined_circuit::SoftwareDefinedCircuit;
use crate::wallet::utils::hash_no_pad_compressed_public_key;
use qed_common_circuit::circuits::{
    l1_secp256k1_signature::L1Secp256K1SignatureCircuit, traits::qstandard::QStandardCircuit,
    zk_signature3::core::QEDBasicZKSignatureCircuit,
};

#[derive(Clone)]
pub struct QEDMemoryWallet<C: GenericConfig<D> + 'static, const D: usize>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub zk_circuit: QEDBasicZKSignatureCircuit<C, D>,
    pub secp_circuit: L1Secp256K1SignatureCircuit<C, D>,
    // figerprint, circuit
    pub software_defined_circuit: Option<SoftwareDefinedCircuit<C, D>>,
    // pub simple_sd_circuits: DashMap<QHashOut<C::F>, SimpleSoftwareDefinedCircuit<C, D>>,
    pub simple_software_defined_circuits:
        SimpleSoftwareDefinedCircuit<C, D, SoftwareDefinedSignGadget>,
    pub zk_public_key_to_private_key_store: DashMap<QHashOut<C::F>, QHashOut<C::F>>,
    pub secp_public_key_to_private_key_store: DashMap<QHashOut<C::F>, QHashOut<C::F>>,
    pub software_defined_public_key_to_private_key_store: DashMap<QHashOut<C::F>, QHashOut<C::F>>,
    pub software_defined_public_key_to_private_key_v2_store:
        DashMap<QHashOut<C::F>, QHashOut<C::F>>,
}

impl<C: GenericConfig<D> + 'static, const D: usize> QEDMemoryWallet<C, D>
where
    C::Hasher:
        MerkleZeroHasher<QHashOut<C::F>> + MerkleZeroHasher<HashOut<C::F>> + AlgebraicHasher<C::F>,
{
    pub fn new() -> Self {
        Self {
            zk_public_key_to_private_key_store: DashMap::new(),
            secp_public_key_to_private_key_store: DashMap::new(),
            software_defined_public_key_to_private_key_store: DashMap::new(),
            software_defined_public_key_to_private_key_v2_store: DashMap::new(),
            software_defined_circuit: None,
            simple_software_defined_circuits: SimpleSoftwareDefinedCircuit::new(),
            zk_circuit: QEDBasicZKSignatureCircuit::new(),
            secp_circuit: L1Secp256K1SignatureCircuit::new(),
        }
    }

    pub fn get_zksig_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.zk_circuit.get_fingerprint()
    }
    pub fn get_secp_circuit_fingerprint(&self) -> QHashOut<C::F> {
        self.secp_circuit.get_fingerprint()
    }

    pub fn add_zk_private_key(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let pk_info = self.get_zk_pk_info(private_key)?;
        self.zk_public_key_to_private_key_store
            .insert(pk_info.public_key_param, private_key);
        Ok(pk_info)
    }
    pub fn get_zk_pk_info(
        &self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let private_key = SimpleQEDPrivateKey { private_key };
        let public_key_param = private_key.get_public_key_param::<C::Hasher>();
        let fingerprint = self.get_zksig_circuit_fingerprint();

        Ok(ZKPublicKeyInfo {
            fingerprint,
            public_key_param,
        })
    }

    pub fn add_secp_private_key(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let pk_info = self.get_secp_pk_info(private_key)?;
        tracing::info!("add secp user {}", serde_json::to_string_pretty(&pk_info)?);

        self.secp_public_key_to_private_key_store
            .insert(pk_info.public_key_param, private_key);
        Ok(pk_info)
    }

    pub fn add_simple_software_defined_private_key(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let public_key_param = QHashOut(SoftwareDefinedSignGadget::get_public_key::<
            <C as GenericConfig<D>>::F,
            C::Hasher,
        >(private_key.into()));
        let pk_info = ZKPublicKeyInfo {
            fingerprint: self.simple_software_defined_circuits.get_fingerprint(),
            public_key_param,
        };
        tracing::info!(
            "add software defined user {}",
            serde_json::to_string_pretty(&pk_info)?
        );

        self.software_defined_public_key_to_private_key_store
            .insert(pk_info.public_key_param, private_key);
        Ok(pk_info)
    }

    pub fn add_simple_software_defined_private_key_v2(
        &mut self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let public_key_param = QHashOut(SoftwareDefinedSignGadget::get_public_key::<
            <C as GenericConfig<D>>::F,
            C::Hasher,
        >(private_key.into()));
        let pk_info = ZKPublicKeyInfo {
            fingerprint: self
                .software_defined_circuit
                .as_ref()
                .ok_or(anyhow::format_err!("register sdc first"))?
                .get_fingerprint(),
            public_key_param,
        };
        tracing::info!(
            "add software defined v2 user {}",
            serde_json::to_string_pretty(&pk_info)?
        );

        self.software_defined_public_key_to_private_key_v2_store
            .insert(pk_info.public_key_param, private_key);
        Ok(pk_info)
    }

    pub fn get_secp_public_key(
        &self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<CompressedPublicKey> {
        super::utils::get_secp_public_key(private_key)
    }
    pub fn get_secp_pk_info(
        &self,
        private_key: QHashOut<C::F>,
    ) -> anyhow::Result<ZKPublicKeyInfo<C::F>> {
        let pub_compressed = self.get_secp_public_key(private_key)?;
        tracing::info!("get secp public key {:?}", pub_compressed);

        let public_key_params =
            hash_no_pad_compressed_public_key::<C::F, PoseidonPermutation<C::F>>(pub_compressed);

        Ok(ZKPublicKeyInfo {
            fingerprint: self.get_secp_circuit_fingerprint(),
            public_key_param: public_key_params,
        })
    }

    pub fn zk_sign_for_public_key(
        &self,
        public_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let private_key = self
                .zk_public_key_to_private_key_store
                .get(&public_key)
                .ok_or(anyhow::format_err!("tried to sign with a public key ({}) which does not match any private keys in the store", public_key.to_string()))?;
        self.zk_circuit.prove_base(*private_key, sig_hash)
    }
    pub fn zk_sign_with_private_key(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.zk_circuit.prove_base(private_key, sig_hash)
    }

    pub fn sdc_sign_for_public_key(
        &self,
        user_contract_state: UserContractState<C::F>,
        public_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let private_key = self
                .software_defined_public_key_to_private_key_store
                .get(&public_key)
                .ok_or(anyhow::format_err!("tried to sign with a public key ({}) which does not match any private keys in the store", public_key.to_string()))?;
        self.simple_software_defined_circuits.prove_base(
            user_contract_state,
            *private_key,
            sig_hash,
        )
    }
    pub fn sdc_sign_with_private_key(
        &self,
        user_contract_state: UserContractState<C::F>,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.simple_software_defined_circuits
            .prove_base(user_contract_state, private_key, sig_hash)
    }

    pub fn secp256k1_sign(
        &self,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
        let signing_key = k256::ecdsa::SigningKey::from_slice(&Hash256::from(private_key).0)?;
        let result: k256::ecdsa::Signature =
            signing_key.sign_prehash(&Hash256::from(sig_hash).0)?;
        let mut rs_bytes = [0u8; 64];

        let r_bytes = result.r().to_bytes();
        let s_bytes = result.s().to_bytes();
        rs_bytes[0..32].copy_from_slice(&r_bytes);
        rs_bytes[32..64].copy_from_slice(&s_bytes);

        Ok(QEDCompressedSecp256K1Signature {
            public_key: self.get_secp_public_key(private_key)?.0,
            signature: rs_bytes,
            message: Hash256::from(sig_hash),
        })
    }
    pub fn secp256k1_sign_with_public_key(
        &self,
        public_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<QEDCompressedSecp256K1Signature> {
        let private_key = self.secp_public_key_to_private_key_store.get(&public_key).
        ok_or(anyhow::format_err!("tried to sign with a public key ({}) which does not match any private keys in the store", public_key.to_string()))?;
        self.secp256k1_sign(*private_key, sig_hash)
    }
    pub fn zk_secp256k1_from_signature(
        &self,
        signature: &QEDCompressedSecp256K1Signature,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        self.secp_circuit.prove(signature)
    }
    pub fn zk_sign_secp256k1(
        &self,
        public_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let ecc_sig = self.secp256k1_sign_with_public_key(public_key, sig_hash)?;
        self.secp_circuit.prove(&ecc_sig)
    }
}

/// software defined circuit
impl<C: GenericConfig<D> + 'static, const D: usize> QEDMemoryWallet<C, D>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn register_software_defined_circuit(
        &mut self,
        circuit_def: DPNFunctionCircuitDefinition,
    ) -> anyhow::Result<()> {
        if self.software_defined_circuit.is_none() {
            self.software_defined_circuit = Some(SoftwareDefinedCircuit::new(
                &circuit_def,
                0,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                false,
            ));
        } else {
            tracing::info!("software defined v2 circuit is already registered")
        }
        Ok(())
    }

    pub fn software_defined_sign(
        &self,
        cfc_input: &DapenContractFunctionCircuitInput<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let circuit = self
            .software_defined_circuit
            .as_ref()
            .ok_or(anyhow::format_err!(
                "software defined circuit is not registered"
            ))?;

        circuit.prove(&cfc_input, sig_hash)
    }
}
