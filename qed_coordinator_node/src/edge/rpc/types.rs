use std::collections::HashMap;
use serde::Deserialize;
use plonky2::{plonk::proof::ProofWithPublicInputs, plonk::config::PoseidonGoldilocksConfig};
use serde::de::DeserializeOwned;
use qed_store::config::store_config::QEDFelt;
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;

#[derive(Deserialize)]
pub struct SubmitGUTAParams {
    pub input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
    pub proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
}

#[derive(Debug, Deserialize)]
#[serde(bound = "")]
pub struct GetUserIdRequest {
    pub public_key_param: String, // hex string
}



#[derive(Debug, Deserialize)]
pub struct GetByIdRequest {
    pub id: u64,
}
#[derive(Debug, Deserialize)]
pub struct GetByFRequest
{
    pub id: QEDFelt,
}

#[derive(Debug, Deserialize)]
pub struct GetUserLeafRequest {
    pub checkpoint_id: u64,
    pub user_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct GetUserRegistrationLeafRequest {
    pub checkpoint_id: u64,
    pub leaf_index: u64,
}
#[derive(Debug, Deserialize)]
pub struct GetUserRegistrationFLeafRequest
{
    pub checkpoint_id: QEDFelt,
    pub leaf_index: QEDFelt,
}

use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;
use serde::{Serialize};
use qed_data::guta::api::GUTARealmCheckpointResult;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;

/// the proof that client upload
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "F: serde::de::DeserializeOwned, C: GenericConfig<D, F = F>")]
pub struct UpdateProofsParams<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub realm_id: String,   // source realm id
    pub checkpoint_id: u64, // checkpoint
    pub proof_id: String,   // ProofID（Maybe QProvingJobDataID）
    pub description: Option<String>, //
    pub timestamp: u64,     //

    pub checkpoint_result: GUTARealmCheckpointResult<F>,
    pub proof_with_public_inputs: ProofWithPublicInputs<F, C, D>,
}

/// push the latest checkpoint sync info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSyncInfo {
    pub checkpoint_id: u64,     // checkpoint id
    pub description: Option<String>,
    pub source_coordinator_edge_id: Option<String>,
    pub sync_timestamp: u64, //
    pub compact: QEDCheckpointSyncInfoCompact<QEDFelt>,
}


#[derive(Debug, Clone, Default)]
pub struct RealmInfo {
    pub name: String,     // "realm-edge-1"
    pub rpc_url: String,  // "http://1.2.3.4:8545"
}

#[derive(Debug, Clone, Default)]
pub struct RealmRpcRegistry {
    pub realms: HashMap<String, RealmInfo>, // key = rpc_url, value = RealmInfo
}


#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRealmRpcRequest {
    pub name: String,
    pub rpc_url: String,
}

