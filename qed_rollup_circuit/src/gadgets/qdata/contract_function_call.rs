use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, iop::{target::Target, witness::Witness}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_common_circuit::{builder::core::CircuitBuilderHelpersCore, traits::{AlgebraicHashableTarget, CreatableTarget, FromTargets, ToTargets, WitnessValueFor}};
use qed_core::config::network_constants::DEFERRED_CALL_MAGIC;
use qed_data::dpn::proving_session::DPNProvingSessionDeferredMethodCall;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DPNProvingSessionDeferredMethodCallGadget {
    pub contract_id: Target,
    pub method_id: Target,
    pub inputs: Vec<Target>,
}

impl DPNProvingSessionDeferredMethodCallGadget {
    pub fn add_virtual_to<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        input_count: usize,
    ) -> Self {
        let contract_id = builder.add_virtual_target();
        let method_id = builder.add_virtual_target();
        let inputs = (0..input_count).map(|_| builder.add_virtual_target()).collect::<Vec<Target>>();

        Self {
            contract_id,
            method_id,
            inputs,
        }
        
    }
    pub fn set_witness<F: RichField>(&self, witness: &mut impl Witness<F>, target: &DPNProvingSessionDeferredMethodCall<F>) {
        witness.set_target(self.contract_id, target.contract_id);
        witness.set_target(self.method_id, target.method_id);
        witness.set_target_arr(&self.inputs, &target.inputs);
    }
    pub fn to_hash<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        let inputs_length = self.inputs.len();
        let inputs_length_target = builder.constant_u64(inputs_length as u64);

        let mut inputs_hash_preimage = Vec::with_capacity(inputs_length+2);

        inputs_hash_preimage.push(inputs_length_target);
        inputs_hash_preimage.extend_from_slice(&self.inputs);
        inputs_hash_preimage.push(inputs_length_target);
        let inputs_hash = builder.hash_n_to_hash_no_pad::<H>(inputs_hash_preimage);

        let magic_felt = builder.constant_u64(DEFERRED_CALL_MAGIC);

        let final_hash = builder.hash_n_to_hash_no_pad::<H>(vec![
            magic_felt,
            self.contract_id,
            self.method_id,
            inputs_length_target,
            inputs_hash.elements[0],
            inputs_hash.elements[1],
            inputs_hash.elements[2],
            inputs_hash.elements[3],
        ]);
        final_hash
    }
}
impl AlgebraicHashableTarget for DPNProvingSessionDeferredMethodCallGadget {
    fn to_hash_target<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(&self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl ToTargets for DPNProvingSessionDeferredMethodCallGadget {
    fn to_targets(&self) -> Vec<Target> {  
        let mut targets = Vec::with_capacity(2+self.inputs.len());
        targets.push(self.contract_id);
        targets.push(self.method_id);
        targets.extend_from_slice(&self.inputs);
        targets
    }
}
impl FromTargets for DPNProvingSessionDeferredMethodCallGadget {
    fn from_targets(targets: &[Target]) -> Self {
        if targets.len() < 2 {
            panic!("Invalid number of elements for DPNProvingSessionDeferredMethodCall");
        }
        Self {
            contract_id: targets[0],
            method_id: targets[1],
            inputs: targets[2..].to_vec()
        }
    }
}


impl<F: RichField> WitnessValueFor<DPNProvingSessionDeferredMethodCallGadget, F, true> for DPNProvingSessionDeferredMethodCall<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &DPNProvingSessionDeferredMethodCallGadget) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<DPNProvingSessionDeferredMethodCallGadget, F, false> for DPNProvingSessionDeferredMethodCall<F> {
    fn set_for_witness(&self, witness: &mut impl Witness<F>, target: &DPNProvingSessionDeferredMethodCallGadget) {
        target.set_witness(witness, self);
    }
}
