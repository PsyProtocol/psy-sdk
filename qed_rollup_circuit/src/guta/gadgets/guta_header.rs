use plonky2::{field::extension::Extendable, hash::hash_types::{HashOutTarget, RichField}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalUserTreeAggregatorHeaderGadget {
    pub guta_circuit_whitelist: HashOutTarget,

}

impl GlobalUserTreeAggregatorHeaderGadget {
    pub fn add_virtual_to<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let guta_circuit_whitelist = builder.add_virtual_hash();


        


        Self {
            guta_circuit_whitelist,
        }
    }
    /*
    pub fn set_witness<F: RichField>(
        &self,
        witness: &mut impl Witness<F>,
        target: &UserProvingSessionHeader<F>,
    ) {
        witness.set_hash_target(
            self.ups_step_circuit_whitelist_root, 
            target.ups_step_circuit_whitelist_root.0,
        );
        self.session_start_context.set_witness(witness, &target.session_start_context);
        self.current_state.set_witness(witness, &target.current_state);
    }
    pub fn to_hash<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> HashOutTarget {
        let start_current_combo = builder.hash_two_to_one::<H>(
            self.session_start_context_hash,
            self.current_state_hash
        );

        builder.hash_two_to_one::<H>(
            self.ups_step_circuit_whitelist_root,
            start_current_combo,
        )
    }*/
}

/* 
impl CreatableWithHasherTarget for GlobalUserTreeAggregatorHeaderGadget {
    fn create_virtual_with_hasher<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        Self::add_virtual_to::<H, F, D>(builder)
    }
}
impl AlgebraicHashableTarget for GlobalUserTreeAggregatorHeaderGadget {
    fn to_hash_target<H:AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> HashOutTarget {
        self.to_hash::<H, F, D>(builder)
    }
}
impl<F: RichField> WitnessValueFor<GlobalUserTreeAggregatorHeaderGadget, F, true>
    for UserProvingSessionHeader<F>
{
    fn set_for_witness(
        &self,
        witness: &mut impl Witness<F>,
        target: &GlobalUserTreeAggregatorHeaderGadget,
    ) {
        target.set_witness(witness, self);
    }
}

impl<F: RichField> WitnessValueFor<GlobalUserTreeAggregatorHeaderGadget, F, false>
    for UserProvingSessionHeader<F>
{
    fn set_for_witness(
        &self,
        witness: &mut impl Witness<F>,
        target: &GlobalUserTreeAggregatorHeaderGadget,
    ) {
        target.set_witness(witness, self);
    }
}
*/
