use plonky2::{field::extension::Extendable, hash::hash_types::RichField, iop::target::{BoolTarget, Target}, plonk::{circuit_builder::CircuitBuilder, config::AlgebraicHasher}};
use qed_core::config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT};

fn reverse_bits_in_limit(x: u64, num_bits: u8) -> u64 {
    let dif = 64 - num_bits as u64;
    (x).reverse_bits() >> dif
}

trait UserIdGeneratorStrategy {
    fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        user_registration_tree_leaf_index: Target,
        user_registration_tree_leaf_index_bits: &[BoolTarget],
        global_user_tree_height: usize,
    )-> Target;
    fn get_user_id_from_registration_id(registration_id: u64) -> u64;
}
struct UserIdBitsStrategy1;

impl UserIdGeneratorStrategy for UserIdBitsStrategy1 {
    fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        _user_registration_tree_leaf_index: Target,
        user_registration_tree_leaf_index_bits: &[BoolTarget],
        _global_user_tree_height: usize,
    )-> Target {
        let mut reversed_bits = user_registration_tree_leaf_index_bits.to_vec();
        reversed_bits.reverse();

        let reversed_bits_index = builder.le_sum(reversed_bits.iter());

        reversed_bits_index
    }

    fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
        let dif = 64 - GLOBAL_USER_TREE_HEIGHT as u64;
        (registration_id).reverse_bits() >> dif
    }
}

struct UserIdBitsStrategy2;

impl UserIdGeneratorStrategy for UserIdBitsStrategy2 {
    fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        _user_registration_tree_leaf_index: Target,
        user_registration_tree_leaf_index_bits: &[BoolTarget],
        _global_user_tree_height: usize,
    )-> Target {
        let mut new_top_bits = user_registration_tree_leaf_index_bits[0..(COORDINATOR_USER_TREE_HEIGHT as usize)].to_vec();
        new_top_bits.reverse();

        let new_bottom_bits = user_registration_tree_leaf_index_bits[(COORDINATOR_USER_TREE_HEIGHT as usize)..].to_vec();




        let new_bits = [
            new_bottom_bits,
            new_top_bits,
        ].concat();
        let reversed_bits_index = builder.le_sum(new_bits.iter());

        reversed_bits_index
    }

    fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
        // rotate realms on each index
        let new_top_bits = reverse_bits_in_limit(registration_id&((1u64<<COORDINATOR_USER_TREE_HEIGHT)-1u64), COORDINATOR_USER_TREE_HEIGHT);

        // sequential within realms
        let new_bottom_bits = registration_id>>COORDINATOR_USER_TREE_HEIGHT;

        (new_top_bits<<REALM_USER_TREE_HEIGHT)|new_bottom_bits
    }
}


struct UserIdBitsStrategy3;

impl UserIdGeneratorStrategy for UserIdBitsStrategy3 {
    fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        _user_registration_tree_leaf_index: Target,
        user_registration_tree_leaf_index_bits: &[BoolTarget],
        _global_user_tree_height: usize,
    )-> Target {

        let mut new_top_bits = user_registration_tree_leaf_index_bits[10..].to_vec();
        new_top_bits.reverse();

        let new_bottom_bits = user_registration_tree_leaf_index_bits[0..10].to_vec();




        let new_bits = [
            new_bottom_bits,
            new_top_bits,
        ].concat();
        let reversed_bits_index = builder.le_sum(new_bits.iter());

        reversed_bits_index
    }

    fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
        (reverse_bits_in_limit(registration_id>>10u64, GLOBAL_USER_TREE_HEIGHT-10)<<10u64) |
        (registration_id & ((1u64<<10)-1u64))
    }
}
/*
// reverse bits gives a very even distribution
pub fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
    let dif = 64 - GLOBAL_USER_TREE_HEIGHT as u64;
    (registration_id).reverse_bits() >> dif
}


pub fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    _user_registration_tree_leaf_index: Target,
    user_registration_tree_leaf_index_bits: &[BoolTarget],
    _global_user_tree_height: usize,
) -> Target {
    let mut reversed_bits = user_registration_tree_leaf_index_bits.to_vec();
    reversed_bits.reverse();

    let reversed_bits_index = builder.le_sum(reversed_bits.iter());

    reversed_bits_index
}
*/

type UserIdBitsStrategy = UserIdBitsStrategy1;

pub fn get_user_id_from_registration_id(registration_id: u64) -> u64 {
    UserIdBitsStrategy::get_user_id_from_registration_id(registration_id)
}
pub fn circuit_user_registration_tree_index_bits_to_user_id<H: AlgebraicHasher<F>, F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    user_registration_tree_leaf_index: Target,
    user_registration_tree_leaf_index_bits: &[BoolTarget],
    global_user_tree_height: usize,
) -> Target {
    UserIdBitsStrategy::circuit_user_registration_tree_index_bits_to_user_id::<H,F,D>(
        builder,
        user_registration_tree_leaf_index,
        user_registration_tree_leaf_index_bits,
        global_user_tree_height,
    )
}

#[cfg(test)]
mod tests {
    use plonky2::{field::{extension::Extendable, types::{Field, PrimeField64}}, hash::hash_types::RichField, iop::{target::{BoolTarget, Target}, witness::{PartialWitness, WitnessWrite}}, plonk::{circuit_builder::CircuitBuilder, circuit_data::{CircuitConfig, CircuitData}, config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig}, proof::ProofWithPublicInputs}};
    use qed_core::config::network_constants::GLOBAL_USER_TREE_HEIGHT;
    use rand::{thread_rng, RngCore};

    use super::{UserIdBitsStrategy1, UserIdBitsStrategy2, UserIdBitsStrategy3, UserIdGeneratorStrategy};

    struct SimpleBitsTester<C: GenericConfig<D>, const D: usize> {
        pub registration_ids: Vec<Target>,
        //pub user_ids: Vec<Target>,
        pub circuit_data: CircuitData<C::F,C,D>,
    }

    impl<C: GenericConfig<D>, const D: usize> SimpleBitsTester<C,D> where C::Hasher: AlgebraicHasher<C::F>{
        pub fn new<UIDGen: UserIdGeneratorStrategy>(count: usize, global_user_tree_height: usize) -> Self {

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<C::F, D>::new(config);
        let registration_ids = builder.add_virtual_targets(count);

        let user_ids = registration_ids.iter().map(|x| {
            let user_registration_tree_leaf_index = *x;
            let user_registration_tree_leaf_index_bits = builder.split_le(user_registration_tree_leaf_index, global_user_tree_height);

            UIDGen::circuit_user_registration_tree_index_bits_to_user_id::<C::Hasher, C::F, D>(&mut builder, user_registration_tree_leaf_index, &user_registration_tree_leaf_index_bits, global_user_tree_height)
        }).collect::<Vec<_>>();

        //builder.register_public_inputs(&registration_ids);
        builder.register_public_inputs(&user_ids);


        let circuit_data = builder.build::<C>();
        Self {
            registration_ids,
            circuit_data,
        }

        }

    pub fn prove_base(
        &self,
        registration_ids: &[u64],
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {

        let mut pw = PartialWitness::<C::F>::new();
        //tracing::info!("agg_fingerprint: {}", agg_fingerprint.to_string());
        //tracing::info!("leaf_fingerprint: {}", leaf_fingerprint.to_string());
        for (t,v) in self.registration_ids.iter().zip(registration_ids.iter()) {
            pw.set_target(*t, C::F::from_noncanonical_u64(*v))?;
        }
        /*
        let inner_proof = self.circuit_data.prove(pw)?;

        self.minifier_chain.prove(&inner_proof)*/
        self.circuit_data.prove(pw)
    }
    pub fn check_strategy<UIDGen: UserIdGeneratorStrategy>(&self, registration_ids: &[u64]) -> anyhow::Result<()>{

        for (user_id, reg_id) in self.prove_base(registration_ids)?.public_inputs.iter().map(|x|x.to_canonical_u64()).zip(registration_ids.iter()) {
            let expected_user_id = UIDGen::get_user_id_from_registration_id(*reg_id);
            if expected_user_id != user_id {
                anyhow::bail!("expected registration_id {} to map to {}, but got {} from the circuit instead", *reg_id, expected_user_id, user_id);
            }

        }

        Ok(())



    }
    pub fn full_check<UIDGen: UserIdGeneratorStrategy>(batch_size: usize, count: usize, rand_count: usize) -> anyhow::Result<()> {
        type C = PoseidonGoldilocksConfig;
        let max_regions = (count/batch_size) as u64;
        let rand_regions =  (rand_count/batch_size) as u64;
        let b_size = batch_size as u64;
        let circ = Self::new::<UIDGen>(batch_size, GLOBAL_USER_TREE_HEIGHT as usize);
        let mask = (1u64<<GLOBAL_USER_TREE_HEIGHT)-1u64;
        for i in 0u64..(max_regions+1) {
            let ids = ((i*b_size)..(((i+1u64)*b_size))).map(|x| x&mask).collect::<Vec<u64>>();
            circ.check_strategy::<UIDGen>(&ids)?;
        }
        for _ in 0u64..rand_regions {
            let ids = (0..batch_size).map(|_| thread_rng().next_u64()&mask).collect::<Vec<_>>();
            circ.check_strategy::<UIDGen>(&ids)?;
        }

        Ok(())
    }

    }

    #[test]
    fn check_strategy_1() {
        SimpleBitsTester::<PoseidonGoldilocksConfig, 2>::full_check::<UserIdBitsStrategy1>(1024, 64*1024, 64*1024).unwrap();
    }
    #[test]
    fn check_strategy_2() {
        SimpleBitsTester::<PoseidonGoldilocksConfig, 2>::full_check::<UserIdBitsStrategy2>(1024, 64*1024, 64*1024).unwrap();
    }
    #[test]
    fn check_strategy_3() {
        SimpleBitsTester::<PoseidonGoldilocksConfig, 2>::full_check::<UserIdBitsStrategy3>(1024, 64*1024, 64*1024).unwrap();
    }
    /*
    #[test]
    fn test_check_a(){
        let mut t = Vec::new();
        for i in 0..100 {
            t.push((
                i,
                UserIdBitsStrategy1::get_user_id_from_registration_id(i),
                UserIdBitsStrategy2::get_user_id_from_registration_id(i),
                UserIdBitsStrategy3::get_user_id_from_registration_id(i)
            ));
        }
        println!("{:#?}",t);
    }

    */
}
