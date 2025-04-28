use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{config::network_constants::COORD_API_REGISTER_USER_CHANNEL_ID, data::qhashout::QHashOut, job::drain_queue::{DrainQueueMetadata, DrainQueueMetadataTagged}};
use serde::{Deserialize, Serialize};

use crate::hash::traits::{hasher::FieldQHasher, qhashable::QFieldHashable};


#[derive(
    Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Eq, Hash
)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct ZKPublicKeyInfo<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub public_key_param: QHashOut<F>,
}
impl<F: RichField> DrainQueueMetadataTagged for ZKPublicKeyInfo<F> {
    fn get_dq_metadata(&self) -> DrainQueueMetadata {

        let num = self.fingerprint.0.elements[0].to_canonical_u64()+self.public_key_param.0.elements[0].to_canonical_u64();

        DrainQueueMetadata {
            channel_id: COORD_API_REGISTER_USER_CHANNEL_ID,
            checkpoint_id: 0,
            item_id: num,
        }

        
    }
}
impl<F: RichField> ZKPublicKeyInfo<F> {
    pub fn to_hash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> QFieldHashable<F> for ZKPublicKeyInfo<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        H::q_two_to_one(self.fingerprint, self.public_key_param)
    }
}

impl<F: RichField> KVQSerializable for ZKPublicKeyInfo<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}
impl<F: RichField> ZKPublicKeyInfo<F> {
    pub fn to_hex_string(&self) -> String {
        let bytes = self.to_bytes().expect("ZKPublicKeyInfo serialization failed");
        hex::encode(bytes)
    }
    pub fn public_key_hex_string(&self) -> String {
        let mut bytes = Vec::new();
        for elem in &self.public_key_param.0.elements {
            bytes.extend_from_slice(&elem.to_canonical_u64().to_le_bytes());
        }
        hex::encode(bytes)
    }
}