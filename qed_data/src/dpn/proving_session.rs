use kvq::traits::KVQSerializable;
use plonky2::hash::hash_types::RichField;
use qed_core::{config::network_constants::DEFERRED_CALL_MAGIC, data::qhashout::QHashOut, traits::to_qfelts::ToQFelts};
use qed_crypto::hash::traits::{hasher::{FieldHasher, FieldQHasher}, qhashable::QFieldHashable};
use serde::{Deserialize, Serialize};

use crate::qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf}, user::QEDUserLeaf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNProvingSessionCheckpointState<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub checkpoint_hash: QHashOut<F>,
    pub checkpoint_id: F,
    pub checkpoint_leaf: QEDCheckpointLeaf<F>,
    pub last_global_tree_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub session_start_user_leaf: QEDUserLeaf<F>,
}




#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNProvingSessionCompactMethodCall<F: RichField> {
    pub contract_id: F,
    pub method_id: F,
    pub inputs_length: F,
    pub inputs_hash: QHashOut<F>,
}
impl<F: RichField> DPNProvingSessionCompactMethodCall<F> {
    pub fn new_from_inputs<H: FieldQHasher<F>>(contract_id: F, method_id: F, inputs: &[F]) -> Self {
        let inputs_length = inputs.len();
        let inputs_length_felt = F::from_canonical_u64(inputs_length as u64);
        let mut inputs_hash_preimage = Vec::with_capacity(inputs_length+2);


        /*
          to prevent length attacks on poseidon, use input length in preimage 
          let inputs_hash = hash([inputs_length, ...inputs, inputs_length])
        */
        inputs_hash_preimage.push(inputs_length_felt);
        inputs_hash_preimage.extend_from_slice(inputs);
        inputs_hash_preimage.push(inputs_length_felt);
        let inputs_hash = H::hash_many(&inputs_hash_preimage);
        Self {
            contract_id,
            method_id,
            inputs_length: inputs_length_felt,
            inputs_hash,
        }
        
        
    }
}

impl<F: RichField> KVQSerializable for DPNProvingSessionCompactMethodCall<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> ToQFelts<F> for DPNProvingSessionCompactMethodCall<F> {
    fn to_qfelts(&self) -> Vec<F> {
        vec![
            self.contract_id,
            self.method_id,
            self.inputs_length,
            self.inputs_hash.0.elements[0],
            self.inputs_hash.0.elements[1],
            self.inputs_hash.0.elements[2],
            self.inputs_hash.0.elements[3],
        ]
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() != 7 {
            panic!("Invalid number of elements for DPNProvingSessionCompactMethodCall, expected 7, got {}",felts.len());
        }

        Self {
            contract_id: felts[0],
            method_id: felts[1],
            inputs_length: felts[2],
            inputs_hash: QHashOut::from_felt_slice(&felts[3..]),
        }
    }
}


impl<F: RichField> QFieldHashable<F> for DPNProvingSessionCompactMethodCall<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {


        let magic_felt = F::from_noncanonical_u64(DEFERRED_CALL_MAGIC);

        /* 
          we need to be able to verify:
            - the contract being called
            - the method called
            - the length of the inputs
            - the hash of the inputs
        */

        H::hash_many(&[
            magic_felt,
            self.contract_id,
            self.method_id,
            self.inputs_length,
            self.inputs_hash.0.elements[0],
            self.inputs_hash.0.elements[1],
            self.inputs_hash.0.elements[2],
            self.inputs_hash.0.elements[3],
        ])
    }
}




#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNProvingSessionSimpleMethodCall<F: RichField> {
    pub contract_id: F,
    pub method_id: F,
    pub inputs: Vec<F>,
}


impl<F: RichField> KVQSerializable for DPNProvingSessionSimpleMethodCall<F> {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow::anyhow!(e))
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<F: RichField> ToQFelts<F> for DPNProvingSessionSimpleMethodCall<F> {
    fn to_qfelts(&self) -> Vec<F> {
        let mut felts = Vec::with_capacity(2+self.inputs.len());
        felts.push(self.contract_id);
        felts.push(self.method_id);
        felts.extend_from_slice(&self.inputs);
        felts
    }

    fn from_qfelts(felts: &[F]) -> Self {
        if felts.len() < 2 {
            panic!("Invalid number of elements for DPNProvingSessionSimpleMethodCall");
        }
        Self {
            contract_id: felts[0],
            method_id: felts[1],
            inputs: felts[2..].to_vec()
        }
    }
}


impl<F: RichField> QFieldHashable<F> for DPNProvingSessionSimpleMethodCall<F> {
    fn qfhash<H: FieldQHasher<F>>(&self) -> QHashOut<F> {
        DPNProvingSessionCompactMethodCall::new_from_inputs::<H>(
            self.contract_id,
            self.method_id,
            &self.inputs
        ).qfhash::<H>()
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNProvingSessionCallStack<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub checkpoint_hash: QHashOut<F>,
    pub checkpoint_id: F,
    pub checkpoint_leaf: QEDCheckpointLeaf<F>,
    pub last_global_tree_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub session_start_user_leaf: QEDUserLeaf<F>,
}


