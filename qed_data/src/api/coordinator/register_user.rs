use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use psy_crypto::hash::traits::{hasher::FieldQHasher, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};


#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash
)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDAPIRegisterUserRequest<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub public_key_param: QHashOut<F>,
}

impl<F: RichField> QEDAPIRegisterUserRequest<F> {
    pub fn to_hash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> QFieldHashable<F> for QEDAPIRegisterUserRequest<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> KVQSerializable for QEDAPIRegisterUserRequest<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}




#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash
)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDAPIRegisterUserRequestForUserId<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub public_key_param: QHashOut<F>,
    pub user_id: F,
}

impl<F: RichField> QEDAPIRegisterUserRequestForUserId<F> {
    pub fn to_hash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> QFieldHashable<F> for QEDAPIRegisterUserRequestForUserId<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> KVQSerializable for QEDAPIRegisterUserRequestForUserId<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}