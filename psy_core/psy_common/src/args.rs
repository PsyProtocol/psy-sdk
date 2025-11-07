// Common argument types shared across the PSY ecosystem

use clap::Args;
use plonky2::hash::hash_types::RichField;
use serde::{Deserialize, Serialize};

use crate::data::qhashout::QHashOut;

// Contract and signing related arguments
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

// Command line arguments for various tools
#[derive(Clone, Debug, Args)]
pub struct InterpreterArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,

    #[clap(short, env, long, value_parser = parse_vec_u64, num_args = 0..)]
    pub parameters: Vec<Vec<u64>>,
}

fn parse_vec_u64(s: &str) -> Result<Vec<u64>, String> {
    s.split(',').map(|num| num.parse::<u64>().map_err(|e| e.to_string())).collect()
}

#[derive(Clone, Debug, Args)]
pub struct CompilerArgs {
    #[clap(short, env, long)]
    pub file: String,

    #[clap(short, env, long, default_value = None)]
    pub contract_name: Option<String>,

    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct TestArgs {
    #[clap(short, env, long)]
    pub file: String,
}
