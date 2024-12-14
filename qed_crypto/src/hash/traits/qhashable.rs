use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;

use super::hasher::FieldHasher;

pub trait QFieldHashable<F: RichField>{
    fn qfhash<H: FieldHasher<QHashOut<F>, F>>(&self) -> QHashOut<F>;
}