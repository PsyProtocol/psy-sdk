// Common argument types shared across the PSY ecosystem

use plonky2::hash::hash_types::RichField;
use serde::{Deserialize, Serialize};

use crate::data::qhashout::QHashOut;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractCallArgs {
    pub contract_id: u64,
    pub method_name: String,
    pub inputs: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum SignType {
    ZKSign,
    SECP256K1Sign,
    SoftwareDefinedSign,
}

impl SignType {
    pub fn from_str(s: &str, _ignore_case: bool) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "zk" => Ok(SignType::ZKSign),
            "secp256k1" => Ok(SignType::SECP256K1Sign),
            "software-defined" => Ok(SignType::SoftwareDefinedSign),
            _ => Err(format!("Unknown sign type: {}", s)),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Hash, Eq, Debug)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct SignData<F: RichField> {
    pub fingerprint: QHashOut<F>,
    pub sign_contract_id: u64,
    pub sign_inputs: Vec<u64>,
}
