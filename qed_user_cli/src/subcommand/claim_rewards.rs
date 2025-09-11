use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use kvq::traits::KVQSerializable;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::HashOut,
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{
    data::{base_types::hash256::Hash256, qhashout::QHashOut},
    job::id::{ProvingJobCircuitType, QProvingJobDataID, VariableHeightRewardMerkleProof, GUTA_REWARDS_TREE_MAX_HEIGHT},
};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::{config::store_config::QEDHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync};
use qed_prover::{
    local::{
        args::{ContractCallArgs, JobInfo, JobLocation, SignType, WorkerJobTracker},
        provider::{RpcConfig, RpcProvider},
    },
    session::{
        utils::{load_jobs_from_tracker_file, parse_job_specs},
        WalletSession,
    },
};
use serde_json::json;
use tracing::{info, warn};

use super::args::ClaimRewardsArgs;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub fn run(args: ClaimRewardsArgs) -> Result<()> {
    info!("Starting claim rewards with checkpoint_id: {}", args.checkpoint_id);

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::from(Hash256::from_hex_string(&args.private_key)?);

    let provider = RpcProvider::new_with_config(&rpc_config)?;
    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let fingerprint = if args.fingerprint.is_some() {
        Some(QHashOut::<F>::from_str(&args.fingerprint.as_ref().unwrap()).map_err(|e| anyhow::format_err!("Failed to parse fingerprint: {}", e))?)
    } else {
        None
    };

    let user_pk_hash = wallet_session.add_user_with_type(private_key, args.sign_type.clone(), fingerprint)?;

    let mut job_infos = if args.jobs.is_empty() {
        load_jobs_from_tracker_file(&user_pk_hash, args.checkpoint_id)?
    } else {
        parse_job_specs(&args.jobs)?
    };

    wallet_session.claim_rewards_with_sign_type(
        user_pk_hash,
        args.checkpoint_id,
        job_infos,
        args.sign_type.clone(),
        fingerprint,
        None,
        vec![],
    )?;

    Ok(())
}
