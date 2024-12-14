use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::{HashOut, RichField};
use qed_core::{data::qhashout::QHashOut, traits::to_qfelts::{QFeltSized, ToQFelts}};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QBCRegisterUser<F: RichField> {
    pub public_key: QHashOut<F>,
}

impl<F: RichField> QBCRegisterUser<F> {
    pub fn new(public_key: QHashOut<F>) -> Self {
        Self {
            public_key,
        }
    }
}


impl<F: RichField> ToQFelts<F> for QBCRegisterUser<F> {
    fn to_qfelts(&self) -> Vec<F> {
        self.public_key.0.elements.to_vec()
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 4 {
            panic!("Invalid number of elements for QBCRegisterUser");
        }
        QBCRegisterUser {
            public_key: QHashOut(HashOut {
                elements: [
                    felts[0],
                    felts[1],
                    felts[2],
                    felts[3],
                ]
            }),
        }
    }
}
impl<F: RichField> KVQSerializable for QBCRegisterUser<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> QFeltSized for QBCRegisterUser<F> {
    fn q_felt_size() -> usize {
        4
    }
}