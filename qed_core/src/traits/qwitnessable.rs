pub trait QWitnessable<T> {
    fn set_q_witness(&mut self, witness: &T);
}