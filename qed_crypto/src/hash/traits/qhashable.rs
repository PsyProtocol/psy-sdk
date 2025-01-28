use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;

use super::hasher::{FieldHasher, FieldQHasher};

pub trait QFieldHashable<F: RichField>{
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F>;
}