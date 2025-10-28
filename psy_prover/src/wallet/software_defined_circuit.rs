use std::fmt::Debug;

use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    gates::gate::GateRef,
    hash::{
        hash_types::{HashOut, HashOutTarget, RichField},
        poseidon::PoseidonHash,
    },
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
    },
};
use psy_common_circuit::{
    builder::{
        hash::core::CircuitBuilderHashCore,
        pad_circuit::{pad_circuit_degree, CircuitBuilderQEDCommonGates},
    },
    circuits::traits::qstandard::QStandardCircuit,
    proof_minifier::pm_chain::QEDProofMinifierChain,
    u32::gates::comparison::ComparisonGate,
};
use psy_core::data::qhashout::QHashOut;
use psy_crypto::{hash::traits::hasher::MerkleZeroHasher, signature::zk::wallet::PRIVATE_KEY_CONSTANTS};
use psy_exec::vm::cfc_input::DapenContractFunctionCircuitInput;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    dpn::vm::compile::QEDContractFunctionBuilderGadget,
    local::provider::RpcProvider,
    wallet::simple_sign::{SoftwareDefinedSignTrait, StateReader, StateReaderGadget},
};

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
pub trait SoftwareDefinedSignature<C: GenericConfig<D>, const D: usize> {
    type Input;
    type WitnessInput;
    async fn add_signature_circuit(builder: &mut CircuitBuilder<C::F, D>, inputs: &Self::Input) -> Self;

    fn get_circuit_builder_input(&self) -> Self::Input;

    async fn set_signature_circuit_witness(&mut self, pw: &mut PartialWitness<C::F>, input: &Self::WitnessInput) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct SoftwareDefinedSignatureCircuit<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignature<C, D>>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub private_key: HashOutTarget,
    pub signature_gadget: S,
    pub sig_hash: HashOutTarget,

    pub circuit_data: CircuitData<C::F, C, D>,

    pub minifier_chain: QEDProofMinifierChain<D, C::F, C>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignature<C, D>> SoftwareDefinedSignatureCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub async fn clone(&self) -> Self {
        Self::new(&self.signature_gadget.get_circuit_builder_input()).await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignature<C, D>> SoftwareDefinedSignatureCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub async fn new(input: &S::Input) -> Self {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let private_key = builder.add_virtual_hash();
        let sig_hash = builder.add_virtual_hash();

        let signature_gadget = S::add_signature_circuit(&mut builder, input).await;

        let public_key_param = get_zk_public_key_param::<C, D>(&mut builder, &private_key);

        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(sig_hash, public_key_param);

        builder.register_public_inputs(&public_inputs_hash.elements);

        builder.add_qed_type_b_common_gates();
        pad_circuit_degree::<C::F, D>(&mut builder, 11);

        let circuit_data = builder.build::<C>();

        let added_gates_for_minifier = [GateRef::new(ComparisonGate::new(32, 16))];

        let minifier_chain =
            QEDProofMinifierChain::<D, C::F, C>::new_add_gates(&circuit_data.verifier_only, &circuit_data.common, 2, Some(&added_gates_for_minifier));

        Self {
            // input,
            private_key,
            sig_hash,
            signature_gadget,
            circuit_data,
            minifier_chain,
        }
    }
    pub async fn prove(
        &mut self,
        private_key: QHashOut<C::F>,
        input: &S::WitnessInput,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::<C::F>::new();

        pw.set_hash_target(self.private_key, private_key.0)?;
        pw.set_hash_target(self.sig_hash, sig_hash.0)?;

        self.signature_gadget.set_signature_circuit_witness(&mut pw, input).await?;

        let inner_proof = self.circuit_data.prove(pw)?;
        let minified_proof = self.minifier_chain.prove(&inner_proof)?;
        Ok(minified_proof)
    }
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignature<C, D>> QStandardCircuit<C, D> for SoftwareDefinedSignatureCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F>,
{
    fn get_fingerprint(&self) -> QHashOut<C::F> {
        QHashOut(self.minifier_chain.get_fingerprint())
    }

    fn get_verifier_config_ref(&self) -> &VerifierOnlyCircuitData<C, D> {
        self.minifier_chain.get_verifier_data()
    }

    fn get_common_circuit_data_ref(&self) -> &CommonCircuitData<C::F, D> {
        self.minifier_chain.get_common_data()
    }
}

#[derive(Debug)]
pub enum SoftwareDefinedSignatureGadget {
    QED(QSoftwareDefinedSignatureGadget),
    PLONKY2(PSoftwareDefinedSignatureGadget),
}

#[derive(Debug)]
pub enum SoftwareDefinedSignatureInput {
    QED(QSoftwareDefinedSignatureInput),
    PLONKY2(PSoftwareDefinedSignatureInput),
}

type C = PoseidonGoldilocksConfig;
type GF = GoldilocksField;
const D: usize = 2;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct QSoftwareDefinedSignatureWitnessInput {
    pub cfc_input: DapenContractFunctionCircuitInput<GF>,
}

#[derive(Debug, Clone)]
pub struct PSoftwareDefinedSignatureWitnessInput {
    pub state_reader: StateReader<GF, D, RpcProvider>,
    pub circuit_inputs: Vec<GF>,
}
#[derive(Debug)]
pub enum SoftwareDefinedSignatureWitnessInput {
    QED(QSoftwareDefinedSignatureWitnessInput),
    PLONKY2(PSoftwareDefinedSignatureWitnessInput),
}

#[derive(Debug, Clone)]
pub struct QSoftwareDefinedSignatureGadget {
    pub fn_builder_gadget: QEDContractFunctionBuilderGadget,
    pub input: QSoftwareDefinedSignatureInput,
    pub circuit_inputs: Vec<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct QSoftwareDefinedSignatureInput {
    pub fn_def: DPNFunctionCircuitDefinition,
    pub contract_id: u64,
    pub contract_state_tree_height: u8,
    pub session_proof_tree_height: u8,
    pub force_four_align: bool,
}

#[derive(Debug, Clone)]
pub struct PSoftwareDefinedSignatureInput {
    // pub contract_id: u64,
    // pub user_id: u64,
    pub contract_state_tree_height: u8,
    // pub session_proof_tree_height: usize,
    pub input_len: usize,
    pub sign_circuit: Box<dyn SoftwareDefinedSignTrait>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl SoftwareDefinedSignature<C, D> for QSoftwareDefinedSignatureGadget {
    type Input = QSoftwareDefinedSignatureInput;
    type WitnessInput = QSoftwareDefinedSignatureWitnessInput;
    async fn add_signature_circuit(builder: &mut CircuitBuilder<GF, D>, input: &Self::Input) -> Self {
        let circuit_inputs = builder.add_virtual_targets(input.fn_def.circuit_inputs.len());
        let fn_builder_gadget = QEDContractFunctionBuilderGadget::add_virtual_to::<PoseidonHash, GF, D>(
            builder,
            &input.fn_def,
            input.contract_state_tree_height as usize,
            input.session_proof_tree_height as usize,
            circuit_inputs.clone(),
            input.force_four_align,
        );

        // enforce user contract state tree root is not change
        let start_contract_state_tree_root = fn_builder_gadget.tx_ctx_header.transaction_call_start_ctx.start_contract_state_tree_root;
        let end_contract_state_tree_root = fn_builder_gadget.tx_ctx_header.transaction_end_ctx.end_contract_state_tree_root;
        builder.connect_hashes(start_contract_state_tree_root, end_contract_state_tree_root);
        Self {
            fn_builder_gadget,
            input: input.clone(),
            circuit_inputs,
        }
    }

    fn get_circuit_builder_input(&self) -> Self::Input {
        self.input.clone()
    }

    async fn set_signature_circuit_witness(&mut self, pw: &mut PartialWitness<GF>, witness_input: &Self::WitnessInput) -> anyhow::Result<()> {
        pw.set_target_arr(&self.circuit_inputs, &witness_input.cfc_input.inputs)?;

        pw.set_hash_target(
            self.fn_builder_gadget.session_proof_tree_root,
            witness_input.cfc_input.session_proof_tree_root.0,
        )?;
        self.fn_builder_gadget
            .tx_ctx_header
            .set_witness(pw, &witness_input.cfc_input.tx_input_ctx)?;
        self.fn_builder_gadget
            .state_reader
            .set_witness(pw, &witness_input.cfc_input, &self.input.fn_def);
        Ok(())
    }
}

#[derive(Debug)]
pub struct PSoftwareDefinedSignatureGadget {
    pub state_reader_gadget: StateReaderGadget<GF, D>,
    pub input: PSoftwareDefinedSignatureInput,
    pub circuit_inputs: Vec<Target>,
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl SoftwareDefinedSignature<C, D> for PSoftwareDefinedSignatureGadget {
    type Input = PSoftwareDefinedSignatureInput;
    type WitnessInput = PSoftwareDefinedSignatureWitnessInput;
    async fn add_signature_circuit(builder: &mut CircuitBuilder<GF, D>, input: &Self::Input) -> Self {
        let circuit_inputs = builder.add_virtual_targets(input.input_len);
        let mut state_reader_gadget = StateReaderGadget::new(builder, input.contract_state_tree_height as u8);
        let mut sign_circuit = input.sign_circuit.clone();
        sign_circuit
            .as_mut()
            .custom_sign_option_f(builder, &mut state_reader_gadget, circuit_inputs.clone())
            .await
            .expect("custom sign option failed");

        Self {
            state_reader_gadget,
            input: input.clone(),
            circuit_inputs,
        }
    }

    fn get_circuit_builder_input(&self) -> Self::Input {
        self.input.clone()
    }

    async fn set_signature_circuit_witness(&mut self, pw: &mut PartialWitness<GF>, witness_input: &Self::WitnessInput) -> anyhow::Result<()> {
        pw.set_target_arr(&self.circuit_inputs, &witness_input.circuit_inputs)?;

        let mut state_reader = witness_input.state_reader.clone();

        self.input
            .sign_circuit
            .as_mut()
            .custom_sign_option(&mut state_reader, witness_input.circuit_inputs.clone())
            .await?;

        self.state_reader_gadget.set_witness(pw, &state_reader)?;
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl SoftwareDefinedSignature<C, D> for SoftwareDefinedSignatureGadget {
    type Input = SoftwareDefinedSignatureInput;

    type WitnessInput = SoftwareDefinedSignatureWitnessInput;

    async fn add_signature_circuit(builder: &mut CircuitBuilder<GF, D>, inputs: &Self::Input) -> Self {
        match inputs {
            SoftwareDefinedSignatureInput::QED(input) => {
                let gadget = QSoftwareDefinedSignatureGadget::add_signature_circuit(builder, input).await;
                Self::QED(gadget)
            }
            SoftwareDefinedSignatureInput::PLONKY2(input) => {
                let gadget = PSoftwareDefinedSignatureGadget::add_signature_circuit(builder, input).await;
                Self::PLONKY2(gadget)
            }
        }
    }

    fn get_circuit_builder_input(&self) -> Self::Input {
        match self {
            Self::QED(gadget) => SoftwareDefinedSignatureInput::QED(gadget.get_circuit_builder_input()),
            Self::PLONKY2(gadget) => SoftwareDefinedSignatureInput::PLONKY2(gadget.get_circuit_builder_input()),
        }
    }

    async fn set_signature_circuit_witness(&mut self, pw: &mut PartialWitness<GF>, input: &Self::WitnessInput) -> anyhow::Result<()> {
        match (self, input) {
            (Self::QED(gadget), SoftwareDefinedSignatureWitnessInput::QED(input)) => {
                gadget.set_signature_circuit_witness(pw, input).await?;
            }
            (Self::PLONKY2(gadget), SoftwareDefinedSignatureWitnessInput::PLONKY2(input)) => {
                gadget.set_signature_circuit_witness(pw, input).await?;
            }
            _ => anyhow::bail!("invalid input type"),
        }
        Ok(())
    }
}

pub fn get_zk_public_key_param<C: GenericConfig<D>, const D: usize>(
    builder: &mut CircuitBuilder<C::F, D>,
    private_key: &HashOutTarget,
) -> HashOutTarget
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    let private_key_constants = PRIVATE_KEY_CONSTANTS
        .iter()
        .map(|c| builder.constant(C::F::from_canonical_u64(*c)))
        .collect::<Vec<_>>();
    builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![
        private_key_constants[0],
        private_key_constants[1],
        private_key_constants[2],
        private_key_constants[19],
        private_key.elements[1],
        private_key_constants[1],
        private_key_constants[2],
        private_key_constants[3],
        private_key_constants[4],
        private_key_constants[5],
        private_key_constants[6],
        private_key.elements[0],
        private_key_constants[7],
        private_key.elements[2],
        private_key_constants[8],
        private_key_constants[9],
        private_key_constants[10],
        private_key_constants[11],
        private_key_constants[12],
        private_key.elements[3],
        private_key_constants[13],
        private_key_constants[14],
        private_key_constants[15],
        private_key_constants[16],
        private_key_constants[17],
        private_key_constants[18],
    ])
}

pub fn get_sdc_public_key_param<F: RichField>(private_key: &QHashOut<F>) -> QHashOut<F> {
    let private_key_constants = PRIVATE_KEY_CONSTANTS.iter().map(|c| F::from_canonical_u64(*c)).collect::<Vec<_>>();
    QHashOut(PoseidonHash::hash_no_pad(&[
        private_key_constants[0],
        private_key_constants[1],
        private_key_constants[2],
        private_key_constants[19],
        private_key.0.elements[1],
        private_key_constants[1],
        private_key_constants[2],
        private_key_constants[3],
        private_key_constants[4],
        private_key_constants[5],
        private_key_constants[6],
        private_key.0.elements[0],
        private_key_constants[7],
        private_key.0.elements[2],
        private_key_constants[8],
        private_key_constants[9],
        private_key_constants[10],
        private_key_constants[11],
        private_key_constants[12],
        private_key.0.elements[3],
        private_key_constants[13],
        private_key_constants[14],
        private_key_constants[15],
        private_key_constants[16],
        private_key_constants[17],
        private_key_constants[18],
    ]))
}
