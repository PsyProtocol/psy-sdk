use plonky2::{field::{goldilocks_field::GoldilocksField, types::Field}, hash::{hash_types::HashOut, hashing::PlonkyPermutation, poseidon::PoseidonPermutation}};
use qed_crypto::hash::traits::hasher::{MerkleZeroHasher, PoseidonHasher};


/// A one-way compression function which takes two ~256 bit inputs and returns a ~256 bit output.
pub fn compress_fast_hash<F: Field, P: PlonkyPermutation<F>>(x: &[F; 4], y: &[F; 4]) -> [F; 4] {
    // TODO: With some refactoring, this function could be implemented as
    // hash_n_to_m_no_pad(chain(x.elements, y.elements), NUM_HASH_OUT_ELTS).


    let mut perm = P::new(core::iter::repeat(F::ZERO));
    perm.set_from_slice(x, 0);
    perm.set_from_slice(y, 4);

    perm.permute();
    let elm = perm.squeeze();

    [
        elm[0],
        elm[1],
        elm[2],
        elm[3],
    ]
}


pub trait FastTwoToOne<F: Field> {
    fn fast_two_to_one(x: &[F; 4], y: &[F; 4]) -> [F; 4];
    fn fast_get_zero_hash(reverse_level: usize) -> [F; 4];
}



impl FastTwoToOne<GoldilocksField> for PoseidonHasher {
    fn fast_two_to_one(x: &[GoldilocksField; 4], y: &[GoldilocksField; 4]) -> [GoldilocksField; 4] {
        compress_fast_hash::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(x, y)
    }

    fn fast_get_zero_hash(reverse_level: usize) -> [GoldilocksField; 4] {
        let h: HashOut<GoldilocksField> = PoseidonHasher::get_zero_hash(reverse_level);
        h.elements
    }
}