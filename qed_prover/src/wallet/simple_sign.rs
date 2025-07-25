use plonky2::{
    gates::gate::GateRef,
    hash::hash_types::{HashOut, HashOutTarget, RichField},
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData},
        config::{AlgebraicHasher, GenericConfig},
        proof::ProofWithPublicInputs,
    },
};
use qed_common_circuit::{
    builder::{comparison::CircuitBuilderComparison, hash::core::CircuitBuilderHashCore},
    circuits::traits::qstandard::QStandardCircuit,
    proof_minifier::pm_chain::QEDProofMinifierChain,
    u32::gates::comparison::ComparisonGate,
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::hasher::MerkleZeroHasher;
use qed_data::qdata::user_contract_state::UserContractState;
use qed_rollup_circuit::gadgets::qdata::user_contract_state::UserContractStateGadget;

pub trait SoftwareDefinedSignTrait {
    fn get_public_key_f<C: GenericConfig<D>, const D: usize>(
        builder: &mut CircuitBuilder<C::F, D>,
        state: UserContractStateGadget,
        private_key: HashOutTarget,
    ) -> HashOutTarget
    where
        C::Hasher: AlgebraicHasher<C::F>;

    fn get_public_key<F: RichField, H: AlgebraicHasher<F>>(private_key: HashOut<F>) -> HashOut<F>;
}

#[derive(Debug)]
pub struct SoftwareDefinedSignGadget {
    pub private_key: HashOutTarget,
}

impl SoftwareDefinedSignTrait for SoftwareDefinedSignGadget {
    fn get_public_key_f<C: GenericConfig<D>, const D: usize>(
        builder: &mut CircuitBuilder<C::F, D>,
        state: UserContractStateGadget,
        private_key: HashOutTarget,
    ) -> HashOutTarget
    where
        C::Hasher: AlgebraicHasher<C::F>,
    {
        let one = builder.one();
        let two = builder.add(one, one);
        let user_leaf = state.user_leaf;
        builder.ensure_is_less_than(32, user_leaf.nonce, two);
        let element3 = builder.add(one, private_key.elements[3]);
        builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![
            private_key.elements[0],
            private_key.elements[1],
            private_key.elements[2],
            element3,
        ])
    }

    fn get_public_key<F: RichField, H: AlgebraicHasher<F>>(private_key: HashOut<F>) -> HashOut<F> {
        H::hash_no_pad(&[
            private_key.elements[0],
            private_key.elements[1],
            private_key.elements[2],
            F::from_noncanonical_u64(1) + private_key.elements[3],
        ])
    }
}

#[derive(Debug)]
pub struct SimpleSoftwareDefinedCircuit<
    C: GenericConfig<D>,
    const D: usize,
    S: SoftwareDefinedSignTrait,
> where
    C::Hasher: AlgebraicHasher<C::F>,
{
    pub minifier_chain: QEDProofMinifierChain<D, C::F, C>,
    pub circuit_data: CircuitData<C::F, C, D>,
    pub private_key: HashOutTarget,
    pub public_key_param: HashOutTarget,
    pub sig_hash: HashOutTarget,

    pub user_contract_state: UserContractStateGadget,
    // pub sig_inputs: Vec<Target>,
    // pub simple_sign_gadget: S,
    _marker: std::marker::PhantomData<S>,
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait> Clone
    for SimpleSoftwareDefinedCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait>
    SimpleSoftwareDefinedCircuit<C, D, S>
where
    C::Hasher: AlgebraicHasher<C::F> + MerkleZeroHasher<HashOut<C::F>>,
{
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_zk_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);

        let private_key = builder.add_virtual_hash();

        let user_contract_state = UserContractStateGadget::add_virtual_to(&mut builder);

        // gadget
        let public_key_param =
            S::get_public_key_f::<C, D>(&mut builder, user_contract_state, private_key);

        let sig_hash = builder.add_virtual_hash();
        let public_inputs_hash = builder.hash_two_to_one::<C::Hasher>(sig_hash, public_key_param);
        builder.register_public_inputs(&public_inputs_hash.elements);
        let circuit_data = builder.build::<C>();

        let added_gates_for_minifier = [GateRef::new(ComparisonGate::new(32, 16))];

        let minifier_chain = QEDProofMinifierChain::<D, C::F, C>::new_add_gates(
            &circuit_data.verifier_only,
            &circuit_data.common,
            2,
            Some(&added_gates_for_minifier),
        );

        Self {
            circuit_data,
            sig_hash,
            private_key,
            public_key_param,
            user_contract_state,
            minifier_chain,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn prove_base(
        &self,
        user_contract_state: UserContractState<C::F>,
        private_key: QHashOut<C::F>,
        sig_hash: QHashOut<C::F>,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let mut pw = PartialWitness::new();
        pw.set_hash_target(self.private_key, private_key.0)?;
        pw.set_hash_target(self.sig_hash, sig_hash.0)?;
        self.user_contract_state
            .set_witness(&mut pw, &user_contract_state);
        let inner_proof = self.circuit_data.prove(pw)?;
        self.minifier_chain.prove(&inner_proof)
    }
}

impl<C: GenericConfig<D>, const D: usize, S: SoftwareDefinedSignTrait> QStandardCircuit<C, D>
    for SimpleSoftwareDefinedCircuit<C, D, S>
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
