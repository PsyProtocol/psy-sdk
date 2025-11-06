use anyhow;
use maybe_async::maybe_async;
use plonky2::{
    hash::hash_types::HashOut,
    plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::data::qhashout::QHashOut;
use psy_crypto::{
    common::witnesses::qrecursion::proof_data::AggProofRecord,
    hash::traits::{hasher::MerkleZeroHasher, qhashable::QFieldHashable},
    signature::secp256k1::core::PsyCompressedSecp256K1Signature,
};
use psy_data::{
    qdata::contract::ContractCodeDefinition,
    qstore::controllers::session_info::SessionCircuitInfoStore,
    qstore::imm::cmd_processor::PsyReadCommandProcessorSync,
    traits::qdatastore::qtreedata::PsyComboDataStoreReaderSync,
    ups::{
        start_step::UPSStartStepInput,
        ups_cfc_standard_step::{UPSCFCDeferredTransactionCircuitInput, UPSCFCStandardTransactionCircuitInput},
        ups_end_cap::UPSEndCapFromProofTreeGadgetInput,
    },
};
use crate::{
    vm::cfc_input::DapenContractFunctionCircuitInput,
    ups::signature::{DPNSoftwareDefinedSignatureInput, Plonky2SoftwareDefinedSignatureInput},
    dpn::vm::def::DPNFunctionCircuitDefinition,
};

// Generic trait for UPS circuit managers - will be implemented by different providers
#[cfg_attr(not(target_arch = "wasm32"), maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async(?Send))]
pub trait UPSCircuitManagerTrait<C: GenericConfig<D>, const D: usize>: Send + Sync
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{

    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>);
    async fn prove_ups_start(&self, input: &UPSStartStepInput<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn register_contract_circuits(&self, contract_id: u64, contract_code: &ContractCodeDefinition) -> anyhow::Result<()>;

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64>;

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)>;

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_zk_sign(&self, private_key: QHashOut<C::F>, sig_hash: QHashOut<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_secp_sign(&self, signature: PsyCompressedSecp256K1Signature) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn register_dpn_software_defined_circuit(
        &self,
        fn_def: DPNFunctionCircuitDefinition,
        contract_id: u64,
        contract_state_tree_height: u8,
        session_proof_tree_height: u8,
        force_four_align: bool,
    ) -> anyhow::Result<QHashOut<C::F>>;

    async fn register_plonky2_software_defined_circuit(
        &self,
        contract_state_tree_height: u8,
        input_len: usize,
    ) -> anyhow::Result<QHashOut<C::F>>;

    async fn prove_dpn_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: DPNSoftwareDefinedSignatureInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    // TODO: This method is temporarily commented out due to StateReader serialization issues
    // async fn prove_plonky2_software_defined_sign(
    //     &self,
    //     fingerprint: QHashOut<C::F>,
    //     private_key: QHashOut<C::F>,
    //     input: Plonky2SoftwareDefinedSignatureInput<Self::Store>,
    //     sig_hash: QHashOut<C::F>,
    // ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_start_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_cfc_standard_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_cfc_deferred_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn ups_end_cap_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>>;

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>>;
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize, T> UPSCircuitManagerTrait<C, D> for &T
where
    T: UPSCircuitManagerTrait<C, D> + Sync,
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{

    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        (**self).register_info(info_store).await
    }

    async fn prove_ups_start(&self, input: &UPSStartStepInput<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_start(input).await
    }

    async fn register_contract_circuits(&self, contract_id: u64, contract_code: &ContractCodeDefinition) -> anyhow::Result<()> {
        (**self).register_contract_circuits(contract_id, contract_code).await
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64> {
        (**self).get_method_id(contract_id, method_name).await
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        (**self).get_contract_method_common_data(contract_id, method_id).await
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_contract_call(contract_id, method_id, input).await
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_cfc_standard_tx(input).await
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_cfc_deferred_tx(input).await
    }

    async fn prove_zk_sign(&self, private_key: QHashOut<C::F>, sig_hash: QHashOut<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_zk_sign(private_key, sig_hash).await
    }

    async fn prove_secp_sign(&self, signature: PsyCompressedSecp256K1Signature) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_secp_sign(signature).await
    }

    async fn register_dpn_software_defined_circuit(
        &self,
        fn_def: DPNFunctionCircuitDefinition,
        contract_id: u64,
        contract_state_tree_height: u8,
        session_proof_tree_height: u8,
        force_four_align: bool,
    ) -> anyhow::Result<QHashOut<C::F>> {
        (**self).register_dpn_software_defined_circuit(fn_def, contract_id, contract_state_tree_height, session_proof_tree_height, force_four_align).await
    }

    async fn register_plonky2_software_defined_circuit(
        &self,
        contract_state_tree_height: u8,
        input_len: usize,
    ) -> anyhow::Result<QHashOut<C::F>> {
        (**self).register_plonky2_software_defined_circuit(contract_state_tree_height, input_len).await
    }

    async fn prove_dpn_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: DPNSoftwareDefinedSignatureInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_dpn_software_defined_sign(fingerprint, private_key, input, sig_hash).await
    }

    // TODO: This method is temporarily commented out due to StateReader serialization issues
    // async fn prove_plonky2_software_defined_sign(
    //     &self,
    //     fingerprint: QHashOut<C::F>,
    //     private_key: QHashOut<C::F>,
    //     input: Plonky2SoftwareDefinedSignatureInput<Self::Store>,
    //     sig_hash: QHashOut<C::F>,
    // ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
    //     (**self).prove_plonky2_software_defined_sign(fingerprint, private_key, input, sig_hash).await
    // }

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self)
            .prove_ups_end_cap(circuit_info, end_cap_from_proof_tree_input, agg_proof_record)
            .await
    }

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_start_circuit_fingerprint().await
    }

    async fn ups_start_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_start_circuit_verifier_config().await
    }

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_standard_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_standard_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_cfc_standard_tx_circuit_verifier_config().await
    }

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_deferred_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_deferred_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_cfc_deferred_tx_circuit_verifier_config().await
    }

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_end_cap_circuit_fingerprint().await
    }

    async fn ups_end_cap_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_end_cap_circuit_verifier_config().await
    }

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_circuit_whitelist_root().await
    }

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).zk_circuit_fingerprint().await
    }

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).zk_circuit_verifier_config().await
    }

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).secp_circuit_fingerprint().await
    }

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).secp_circuit_verifier_config().await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize> UPSCircuitManagerTrait<C, D> for Box<dyn UPSCircuitManagerTrait<C, D>>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>> + MerkleZeroHasher<QHashOut<C::F>>,
{
    async fn register_info(&self, info_store: &mut SessionCircuitInfoStore<C::F>) {
        (**self).register_info(info_store).await
    }

    async fn prove_ups_start(&self, input: &UPSStartStepInput<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_start(input).await
    }

    async fn register_contract_circuits(&self, contract_id: u64, contract_code: &ContractCodeDefinition) -> anyhow::Result<()> {
        (**self).register_contract_circuits(contract_id, contract_code).await
    }

    async fn get_method_id(&self, contract_id: u64, method_name: String) -> anyhow::Result<u64> {
        (**self).get_method_id(contract_id, method_name).await
    }

    async fn get_contract_method_common_data(
        &self,
        contract_id: u64,
        method_id: u32,
    ) -> anyhow::Result<(QHashOut<C::F>, VerifierOnlyCircuitData<C, D>)> {
        (**self).get_contract_method_common_data(contract_id, method_id).await
    }

    async fn prove_contract_call(
        &self,
        contract_id: u64,
        method_id: u32,
        input: &DapenContractFunctionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_contract_call(contract_id, method_id, input).await
    }

    async fn prove_ups_cfc_standard_tx(
        &self,
        input: &UPSCFCStandardTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_cfc_standard_tx(input).await
    }

    async fn prove_ups_cfc_deferred_tx(
        &self,
        input: &UPSCFCDeferredTransactionCircuitInput<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_ups_cfc_deferred_tx(input).await
    }

    async fn prove_zk_sign(&self, private_key: QHashOut<C::F>, sig_hash: QHashOut<C::F>) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_zk_sign(private_key, sig_hash).await
    }

    async fn prove_secp_sign(&self, signature: PsyCompressedSecp256K1Signature) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_secp_sign(signature).await
    }

    async fn register_dpn_software_defined_circuit(
        &self,
        fn_def: DPNFunctionCircuitDefinition,
        contract_id: u64,
        contract_state_tree_height: u8,
        session_proof_tree_height: u8,
        force_four_align: bool,
    ) -> anyhow::Result<QHashOut<C::F>> {
        (**self).register_dpn_software_defined_circuit(fn_def, contract_id, contract_state_tree_height, session_proof_tree_height, force_four_align).await
    }

    async fn register_plonky2_software_defined_circuit(
        &self,
        contract_state_tree_height: u8,
        input_len: usize,
    ) -> anyhow::Result<QHashOut<C::F>> {
        (**self).register_plonky2_software_defined_circuit(contract_state_tree_height, input_len).await
    }

    async fn prove_dpn_software_defined_sign(
        &self,
        fingerprint: QHashOut<C::F>,
        private_key: QHashOut<C::F>,
        input: DPNSoftwareDefinedSignatureInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self).prove_dpn_software_defined_sign(fingerprint, private_key, input, sig_hash).await
    }

    async fn prove_ups_end_cap(
        &self,
        circuit_info: &SessionCircuitInfoStore<C::F>,
        end_cap_from_proof_tree_input: &UPSEndCapFromProofTreeGadgetInput<C::F>,
        agg_proof_record: &AggProofRecord<C, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        (**self)
            .prove_ups_end_cap(circuit_info, end_cap_from_proof_tree_input, agg_proof_record)
            .await
    }

    async fn ups_start_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_start_circuit_fingerprint().await
    }

    async fn ups_start_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_start_circuit_verifier_config().await
    }

    async fn ups_cfc_standard_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_standard_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_standard_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_cfc_standard_tx_circuit_verifier_config().await
    }

    async fn ups_cfc_deferred_tx_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_cfc_deferred_tx_circuit_fingerprint().await
    }

    async fn ups_cfc_deferred_tx_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_cfc_deferred_tx_circuit_verifier_config().await
    }

    async fn ups_end_cap_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_end_cap_circuit_fingerprint().await
    }

    async fn ups_end_cap_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).ups_end_cap_circuit_verifier_config().await
    }

    async fn ups_circuit_whitelist_root(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).ups_circuit_whitelist_root().await
    }

    async fn zk_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).zk_circuit_fingerprint().await
    }

    async fn zk_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).zk_circuit_verifier_config().await
    }

    async fn secp_circuit_fingerprint(&self) -> anyhow::Result<QHashOut<C::F>> {
        (**self).secp_circuit_fingerprint().await
    }

    async fn secp_circuit_verifier_config(&self) -> anyhow::Result<VerifierOnlyCircuitData<C, D>> {
        (**self).secp_circuit_verifier_config().await
    }
}

