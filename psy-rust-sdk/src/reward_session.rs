use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::HashOut,
    plonk::{circuit_data::CommonCircuitData, config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::data::qhashout::QHashOut as ClientHash;
use psy_data::{
    bridge_aggregate::{NetworkConfig, RewardSessionProofFields, GOLDILOCKS_MODULUS, REWARD_SESSION_PROOF_FIELD_COUNT},
    qdata::{checkpoint::{PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf}, user::PsyUserLeaf},
};
use psy_config::network_constants::{CHECKPOINT_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use psy_plonky2_circuits::bridge::circuits::{
    reward_inclusion::RewardTagWitness,
    reward_ledger::RewardLedgerWindowValues,
    reward_session::{RewardLedgerStateValues, RewardSessionCircuit, RewardSessionJobWitness, RewardSessionWitness, REWARD_SESSION_STEP_CAPACITY},
};
use psy_prover::local::bridge_aggregate::{
    prove_reward_session_claim, AggregationContext, RewardSessionClaimRequest, RewardSessionProvingInput,
};
use psy_prover::session::WalletSession;
use psy_vm::{reward_authorization::RewardAuthorizationWitness, ups::multisig::MultisigPolicy};
use serde::{Deserialize, Serialize};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const PROOF_BYTES_LIMIT: usize = 16_777_216;
const REWARD_TAG_HEIGHT: usize = 21;

/// Owned JSON transport for one reward-session step. `RewardSessionWitness`
/// borrows its jobs and predecessor proofs, so this value owns those bytes
/// until `prove_reward_session_claim` returns.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardSessionClaimProvingRequest {
    pub config: String,
    pub registry: String,
    pub services_url: String,
    pub context: AggregationContext,
    pub witness: RewardSessionWitnessTransport,
    pub window: RewardLedgerWindowTransport,
    pub expected_old_root: Hash4Transport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardSessionWitnessTransport {
    pub statement: RewardSessionProofFieldsTransport,
    pub config: String,
    pub economic_domain: Bytes32Transport,
    pub window_id: Bytes32Transport,
    pub start_root: Hash4Transport,
    pub source_checkpoint_id: u32,
    pub end_checkpoint_id: u32,
    #[serde(deserialize_with = "strict_checkpoint_leaf")]
    pub source_leaf: PsyCheckpointLeaf<F>,
    pub source_path: [Hash4Transport; CHECKPOINT_TREE_HEIGHT as usize],
    pub old_state: RewardLedgerStateTransport,
    pub new_state: RewardLedgerStateTransport,
    pub own_state: RewardLedgerStateTransport,
    pub old_summary: Hash4Transport,
    pub old_session_root: Hash4Transport,
    pub session_siblings: [Hash4Transport; 32],
    pub own_siblings: [Hash4Transport; 32],
    pub ledger_siblings: Vec<Hash4Transport>,
    pub own_previous: Option<String>,
    pub global_previous: Option<String>,
    pub jobs: Vec<RewardSessionJobTransport>,
    pub is_final_step: bool,
    #[serde(deserialize_with = "strict_checkpoint_leaf")]
    pub end_leaf: PsyCheckpointLeaf<F>,
    pub end_path: [Hash4Transport; CHECKPOINT_TREE_HEIGHT as usize],
    #[serde(deserialize_with = "strict_checkpoint_roots")]
    pub end_roots: PsyCheckpointGlobalStateRoots<F>,
    #[serde(deserialize_with = "strict_user_leaf")]
    pub user_leaf: PsyUserLeaf<F>,
    pub user_path: Vec<Hash4Transport>,
    pub public_key_param: Hash4Transport,
    pub authorization: Option<RewardAuthorizationTransport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardSessionProofFieldsTransport {
    pub checkpoint_tree_root: Hash4Transport,
    pub user_id: u32,
    pub recipient: [u32; 8],
    pub total_amount: [u32; 8],
    pub count: u32,
    pub jobs_commitment: Hash4Transport,
    pub old_ledger_state_root: Hash4Transport,
    pub new_ledger_state_root: Hash4Transport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardLedgerStateTransport {
    pub ledger_window_hash: Hash4Transport,
    pub ledger_root: Hash4Transport,
    pub user_root: Hash4Transport,
    pub session_count: u32,
    pub unfinished_session_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardLedgerWindowTransport {
    pub config_hash: Bytes32Transport,
    pub economic_domain: Bytes32Transport,
    pub window_id: Bytes32Transport,
    pub end_checkpoint_id: u32,
    pub end_checkpoint_root: Hash4Transport,
    pub start_root: Hash4Transport,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardSessionJobTransport {
    pub height: u8,
    pub path_index: u32,
    pub tag: RewardTagTransport,
    pub nullifier_siblings: Vec<Hash4Transport>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RewardTagTransport {
    pub tag_preimage: Hash4Transport,
    pub leaf_left: Hash4Transport,
    pub leaf_right: Hash4Transport,
    pub leaf_tag: Hash4Transport,
    pub siblings: [Hash4Transport; REWARD_TAG_HEIGHT],
    pub parent_tags: [Hash4Transport; REWARD_TAG_HEIGHT],
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "scheme", deny_unknown_fields)]
pub enum RewardAuthorizationTransport {
    Zk { private_key: Hash4Transport },
    Secp { compressed_public_key: Bytes33Transport, signature_rs: Bytes64Transport },
    PersonalSign { compressed_public_key: Bytes33Transport, signature_rs: Bytes64Transport },
    Multisig {
        contract_id: u32,
        #[serde(deserialize_with = "strict_policy")]
        initial_policy: MultisigPolicy,
        policy_slots: [Hash4Transport; 4],
        contract_state_paths: [Vec<Hash4Transport>; 4],
        policy_slot_paths: [[Hash4Transport; 4]; 4],
        member_indices: [u8; 2],
        compressed_public_keys: [Bytes33Transport; 2],
        signatures_rs: [Bytes64Transport; 2],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[String; 4]", into = "[String; 4]")]
pub struct Hash4Transport([u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Bytes32Transport([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Bytes33Transport([u8; 33]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Bytes64Transport([u8; 64]);

struct DecodedRewardSessionWitness {
    statement: RewardSessionProofFields,
    config: NetworkConfig,
    economic_domain: [u8; 32],
    window_id: [u8; 32],
    start_root: [u64; 4],
    source_checkpoint_id: u32,
    end_checkpoint_id: u32,
    source_leaf: PsyCheckpointLeaf<F>,
    source_path: [[u64; 4]; CHECKPOINT_TREE_HEIGHT as usize],
    old_state: RewardLedgerStateValues,
    new_state: RewardLedgerStateValues,
    own_state: RewardLedgerStateValues,
    old_summary: [u64; 4],
    old_session_root: [u64; 4],
    session_siblings: [[u64; 4]; 32],
    own_siblings: [[u64; 4]; 32],
    ledger_siblings: [[u64; 4]; 64],
    own_previous: Option<ProofWithPublicInputs<F, C, 2>>,
    global_previous: Option<ProofWithPublicInputs<F, C, 2>>,
    jobs: Vec<RewardSessionJobWitness>,
    is_final_step: bool,
    end_leaf: PsyCheckpointLeaf<F>,
    end_path: [[u64; 4]; CHECKPOINT_TREE_HEIGHT as usize],
    end_roots: PsyCheckpointGlobalStateRoots<F>,
    user_leaf: PsyUserLeaf<F>,
    user_path: Vec<[u64; 4]>,
    public_key_param: [u64; 4],
    authorization: Option<RewardAuthorizationWitness>,
}

pub fn prove_reward_session_claim_json(
    session: &WalletSession, public_key: ClientHash<F>, request_json: &str,
) -> Result<RewardSessionClaimRequest> {
    let request: RewardSessionClaimProvingRequest = serde_json::from_str(request_json)
        .map_err(|error| anyhow!("reward session request: {error}"))?;
    let config = NetworkConfig::decode(&canonical_base64(&request.config)?)
        .map_err(|error| anyhow!("reward session config: {error}"))?;
    let circuit = RewardSessionCircuit::new(REWARD_SESSION_STEP_CAPACITY, config.chains.len())
        .context("reward session circuit")?;
    let decoded = request.witness.decode(&circuit.circuit_data.common)?;
    let witness = decoded.borrow();
    prove_reward_session_claim(session, public_key, RewardSessionProvingInput {
        config: request.config,
        registry: request.registry,
        services_url: request.services_url,
        context: request.context,
        witness: &witness,
        window: request.window.decode()?,
        expected_old_root: request.expected_old_root.0,
    })
}

impl RewardSessionWitnessTransport {
    fn decode(&self, common: &CommonCircuitData<F, 2>) -> Result<DecodedRewardSessionWitness> {
        anyhow::ensure!(self.jobs.len() <= REWARD_SESSION_STEP_CAPACITY, "reward session job count is outside circuit capacity");
        anyhow::ensure!(self.user_path.len() == GLOBAL_USER_TREE_HEIGHT as usize, "reward session user path length mismatch");
        anyhow::ensure!(self.ledger_siblings.len() == 64, "reward session ledger path length mismatch");
        if self.is_final_step { anyhow::ensure!(self.authorization.is_some(), "final-step authorization missing"); }
        let statement = self.statement.decode()?;
        RewardSessionProofFields::from_public_inputs(&statement.to_public_inputs()
            .map_err(|error| anyhow!("reward session statement: {error}"))?)
            .map_err(|error| anyhow!("reward session statement: {error}"))?;
        Ok(DecodedRewardSessionWitness {
            statement,
            config: NetworkConfig::decode(&canonical_base64(&self.config)?).map_err(|error| anyhow!("reward session witness config: {error}"))?,
            economic_domain: self.economic_domain.0,
            window_id: self.window_id.0,
            start_root: self.start_root.0,
            source_checkpoint_id: self.source_checkpoint_id,
            end_checkpoint_id: self.end_checkpoint_id,
            source_leaf: self.source_leaf,
            source_path: self.source_path.map(|hash| hash.0),
            old_state: self.old_state.decode(),
            new_state: self.new_state.decode(),
            own_state: self.own_state.decode(),
            old_summary: self.old_summary.0,
            old_session_root: self.old_session_root.0,
            session_siblings: self.session_siblings.map(|hash| hash.0),
            own_siblings: self.own_siblings.map(|hash| hash.0),
            ledger_siblings: self.ledger_siblings.iter().map(|hash| hash.0).collect::<Vec<_>>().try_into().unwrap(),
            own_previous: decode_proof(self.own_previous.as_deref(), common)?,
            global_previous: decode_proof(self.global_previous.as_deref(), common)?,
            jobs: self.jobs.iter().map(RewardSessionJobTransport::decode).collect::<Result<_>>()?,
            is_final_step: self.is_final_step,
            end_leaf: self.end_leaf,
            end_path: self.end_path.map(|hash| hash.0),
            end_roots: self.end_roots,
            user_leaf: self.user_leaf,
            user_path: self.user_path.iter().map(|hash| hash.0).collect(),
            public_key_param: self.public_key_param.0,
            authorization: self.authorization.as_ref().map(RewardAuthorizationTransport::decode).transpose()?,
        })
    }
}

impl DecodedRewardSessionWitness {
    fn borrow(&self) -> RewardSessionWitness<'_> {
        RewardSessionWitness {
            statement: self.statement,
            config: self.config.clone(),
            economic_domain: self.economic_domain,
            window_id: self.window_id,
            start_root: self.start_root,
            source_checkpoint_id: self.source_checkpoint_id,
            end_checkpoint_id: self.end_checkpoint_id,
            source_leaf: self.source_leaf,
            source_path: self.source_path,
            old_state: self.old_state,
            new_state: self.new_state,
            own_state: self.own_state,
            old_summary: self.old_summary,
            old_session_root: self.old_session_root,
            session_siblings: self.session_siblings,
            own_siblings: self.own_siblings,
            ledger_siblings: self.ledger_siblings,
            own_previous: self.own_previous.as_ref(),
            global_previous: self.global_previous.as_ref(),
            jobs: &self.jobs,
            is_final_step: self.is_final_step,
            end_leaf: self.end_leaf,
            end_path: self.end_path,
            end_roots: self.end_roots.clone(),
            user_leaf: self.user_leaf,
            user_path: self.user_path.clone(),
            public_key_param: self.public_key_param,
            authorization: self.authorization.as_ref(),
        }
    }
}

impl RewardSessionProofFieldsTransport {
    fn decode(&self) -> Result<RewardSessionProofFields> {
        if self.recipient[5] != 0 || self.recipient[6] != 0 || self.recipient[7] != 0 {
            return Err(anyhow!("reward session recipient width"));
        }
        Ok(RewardSessionProofFields {
            checkpoint_tree_root: self.checkpoint_tree_root.0,
            user_id: self.user_id,
            recipient: self.recipient,
            total_amount: self.total_amount,
            count: self.count,
            jobs_commitment: self.jobs_commitment.0,
            old_ledger_state_root: self.old_ledger_state_root.0,
            new_ledger_state_root: self.new_ledger_state_root.0,
        })
    }
}

impl RewardLedgerStateTransport {
    fn decode(self) -> RewardLedgerStateValues {
        RewardLedgerStateValues {
            ledger_window_hash: self.ledger_window_hash.0,
            ledger_root: self.ledger_root.0,
            user_root: self.user_root.0,
            session_count: self.session_count,
            unfinished_session_count: self.unfinished_session_count,
        }
    }
}

impl RewardLedgerWindowTransport {
    fn decode(self) -> Result<RewardLedgerWindowValues> {
        Ok(RewardLedgerWindowValues {
            config_hash: self.config_hash.0,
            economic_domain: self.economic_domain.0,
            window_id: self.window_id.0,
            end_checkpoint_id: self.end_checkpoint_id,
            end_checkpoint_root: self.end_checkpoint_root.0,
            start_root: self.start_root.0,
        })
    }
}

impl RewardSessionJobTransport {
    fn decode(&self) -> Result<RewardSessionJobWitness> {
        anyhow::ensure!((2..=21).contains(&self.height), "reward session job height out of range");
        let width = u32::from(self.height) - 2;
        let bound = 1u32.checked_shl(width).context("reward session job index bound overflow")?;
        anyhow::ensure!(self.path_index < bound, "reward session job index out of range");
        anyhow::ensure!(self.nullifier_siblings.len() == 63, "reward session job path is not 63 siblings");
        Ok(RewardSessionJobWitness {
            height: self.height,
            path_index: self.path_index,
            tag: self.tag.decode(),
            nullifier_siblings: self.nullifier_siblings.iter().map(|hash| hash.0).collect::<Vec<_>>().try_into().unwrap(),
        })
    }
}

impl RewardTagTransport {
    fn decode(&self) -> RewardTagWitness {
        let hash = |value: Hash4Transport| parth_hash(value.0);
        RewardTagWitness {
            tag_preimage: hash(self.tag_preimage),
            leaf_left: hash(self.leaf_left),
            leaf_right: hash(self.leaf_right),
            leaf_tag: hash(self.leaf_tag),
            siblings: self.siblings.map(hash),
            parent_tags: self.parent_tags.map(hash),
        }
    }
}

impl RewardAuthorizationTransport {
    fn decode(&self) -> Result<RewardAuthorizationWitness> {
        let hash = |value: Hash4Transport| ClientHash(HashOut { elements: value.0.map(F::from_canonical_u64) });
        match self {
            Self::Zk { private_key } => Ok(RewardAuthorizationWitness::Zk { private_key: hash(*private_key) }),
            Self::Secp { compressed_public_key, signature_rs } => Ok(RewardAuthorizationWitness::Secp {
                compressed_public_key: compressed_public_key.0, signature_rs: signature_rs.0,
            }),
            Self::PersonalSign { compressed_public_key, signature_rs } => Ok(RewardAuthorizationWitness::PersonalSign {
                compressed_public_key: compressed_public_key.0, signature_rs: signature_rs.0,
            }),
            Self::Multisig { contract_id, initial_policy, policy_slots, contract_state_paths,
                policy_slot_paths, member_indices, compressed_public_keys, signatures_rs } => {
                initial_policy.validate()?;
                anyhow::ensure!(*contract_id == 6, "multisig requires the policy precompile");
                anyhow::ensure!(
                    member_indices[0] < member_indices[1] && member_indices[1] < 3,
                    "multisig requires two ordered current-member signatures",
                );
                anyhow::ensure!(
                    contract_state_paths.iter().all(|path| path.len() == GLOBAL_CONTRACT_TREE_HEIGHT as usize),
                    "multisig policy contract path height mismatch",
                );
                Ok(RewardAuthorizationWitness::Multisig {
                    contract_id: *contract_id,
                    initial_policy: initial_policy.clone(),
                    policy_slots: policy_slots.map(hash),
                    contract_state_paths: contract_state_paths.clone().map(|path| path.into_iter().map(hash).collect()),
                    policy_slot_paths: policy_slot_paths.map(|path| path.map(hash)),
                    member_indices: *member_indices,
                    compressed_public_keys: compressed_public_keys.map(|key| key.0),
                    signatures_rs: signatures_rs.map(|signature| signature.0),
                })
            }
        }
    }
}

fn decode_proof(value: Option<&str>, common: &CommonCircuitData<F, 2>) -> Result<Option<ProofWithPublicInputs<F, C, 2>>> {
    let Some(value) = value else { return Ok(None) };
    let bytes = canonical_base64(value)?;
    anyhow::ensure!((1..=PROOF_BYTES_LIMIT).contains(&bytes.len()), "reward ledger proof bytes out of range");
    let proof = ProofWithPublicInputs::<F, C, 2>::from_bytes(bytes.clone(), common)
        .map_err(|error| anyhow!("reward session predecessor decoding: {error}"))?;
    anyhow::ensure!(proof.public_inputs.len() == REWARD_SESSION_PROOF_FIELD_COUNT, "reward session predecessor width mismatch");
    anyhow::ensure!(proof.to_bytes() == bytes, "reward ledger proof serialization is not canonical");
    Ok(Some(proof))
}
fn strict_checkpoint_leaf<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PsyCheckpointLeaf<F>, D::Error> {
    Ok(StrictCheckpointLeaf::deserialize(deserializer)?.into())
}
fn strict_checkpoint_roots<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PsyCheckpointGlobalStateRoots<F>, D::Error> {
    Ok(StrictCheckpointRoots::deserialize(deserializer)?.into())
}
fn strict_user_leaf<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<PsyUserLeaf<F>, D::Error> {
    Ok(StrictUserLeaf::deserialize(deserializer)?.into())
}
fn strict_policy<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<MultisigPolicy, D::Error> {
    Ok(StrictPolicy::deserialize(deserializer)?.into())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictCheckpointLeaf {
    global_chain_root: ClientHash<F>,
    stats: StrictCheckpointStats,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictCheckpointStats {
    guta_fees_collected: F, da_fees_collected: F, user_ops_processed: F, total_transactions: F, slots_modified: F,
    pm_jobs_completed: StrictPmJobs, block_time: F, random_seed: ClientHash<F>, pm_rewards_commitment: StrictPmReward,
    da_challenges_claimed: [F; psy_config::network_constants::DA_CHALLENGE_WINDOW],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictPmJobs { deploy_contracts_completed: F, register_users_completed: F, gutas_completed: F }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictPmReward { register_users_root: ClientHash<F>, gutas_root: ClientHash<F>, deploy_contracts_root: ClientHash<F> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictCheckpointRoots {
    contract_tree_root: ClientHash<F>, deposit_tree_root: ClientHash<F>, user_tree_root: ClientHash<F>,
    withdrawal_tree_root: ClientHash<F>, user_registration_tree_root: ClientHash<F>, validator_tree_root: ClientHash<F>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictUserLeaf {
    public_key: ClientHash<F>, user_state_tree_root: ClientHash<F>, balance: F, nonce: F,
    last_checkpoint_id: F, event_index: F, user_id: F,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictPolicy { version: u32, threshold: u8, member_count: u8, member_hashes: [ClientHash<F>; 8] }
impl From<StrictCheckpointLeaf> for PsyCheckpointLeaf<F> {
    fn from(value: StrictCheckpointLeaf) -> Self {
        Self { global_chain_root: value.global_chain_root, stats: value.stats.into() }
    }
}
impl From<StrictCheckpointStats> for psy_data::qdata::checkpoint::PsyCheckpointLeafStats<F> {
    fn from(value: StrictCheckpointStats) -> Self {
        Self { guta_fees_collected: value.guta_fees_collected, da_fees_collected: value.da_fees_collected,
            user_ops_processed: value.user_ops_processed, total_transactions: value.total_transactions, slots_modified: value.slots_modified,
            pm_jobs_completed: psy_data::qdata::pm_jobs_completed_stats::PMJobsCompletedStats { deploy_contracts_completed: value.pm_jobs_completed.deploy_contracts_completed, register_users_completed: value.pm_jobs_completed.register_users_completed, gutas_completed: value.pm_jobs_completed.gutas_completed },
            block_time: value.block_time, random_seed: value.random_seed,
            pm_rewards_commitment: psy_data::qdata::pm_reward_commitment::PMRewardCommitment { register_users_root: value.pm_rewards_commitment.register_users_root, gutas_root: value.pm_rewards_commitment.gutas_root, deploy_contracts_root: value.pm_rewards_commitment.deploy_contracts_root },
            da_challenges_claimed: value.da_challenges_claimed }
    }
}
impl From<StrictCheckpointRoots> for PsyCheckpointGlobalStateRoots<F> {
    fn from(value: StrictCheckpointRoots) -> Self {
        Self { contract_tree_root: value.contract_tree_root, deposit_tree_root: value.deposit_tree_root, user_tree_root: value.user_tree_root,
            withdrawal_tree_root: value.withdrawal_tree_root, user_registration_tree_root: value.user_registration_tree_root, validator_tree_root: value.validator_tree_root }
    }
}
impl From<StrictUserLeaf> for PsyUserLeaf<F> {
    fn from(value: StrictUserLeaf) -> Self {
        Self { public_key: value.public_key, user_state_tree_root: value.user_state_tree_root, balance: value.balance, nonce: value.nonce,
            last_checkpoint_id: value.last_checkpoint_id, event_index: value.event_index, user_id: value.user_id }
    }
}
impl From<StrictPolicy> for MultisigPolicy {
    fn from(value: StrictPolicy) -> Self { Self { version: value.version, threshold: value.threshold, member_count: value.member_count, member_hashes: value.member_hashes } }
}



fn canonical_base64(value: &str) -> Result<Vec<u8>> {
    let bytes = STANDARD.decode(value).map_err(|error| anyhow!("reward session base64: {error}"))?;
    anyhow::ensure!(STANDARD.encode(&bytes) == value, "noncanonical base64");
    Ok(bytes)
}

fn parth_hash(value: [u64; 4]) -> parth_core::pgoldilocks::QHashOut<F> {
    parth_core::pgoldilocks::QHashOut(HashOut { elements: value.map(F::from_canonical_u64) })
}

impl TryFrom<[String; 4]> for Hash4Transport {
    type Error = String;
    fn try_from(value: [String; 4]) -> Result<Self, Self::Error> {
        let mut limbs = [0u64; 4];
        for (limb, text) in limbs.iter_mut().zip(value) {
            if text.is_empty() || text.starts_with('+') || text.starts_with('-') || (text.len() > 1 && text.starts_with('0')) || !text.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("noncanonical reward session hash".to_owned());
            }
            *limb = text.parse::<u64>().map_err(|_| "noncanonical reward session hash".to_owned())?;
            if *limb >= GOLDILOCKS_MODULUS { return Err("noncanonical reward session hash".to_owned()); }
        }
        Ok(Self(limbs))
    }
}
impl From<Hash4Transport> for [String; 4] { fn from(value: Hash4Transport) -> Self { value.0.map(|limb| limb.to_string()) } }

fn fixed_hex<const N: usize>(value: String) -> Result<[u8; N], String> {
    let Some(hex) = value.strip_prefix("0x") else { return Err(format!("canonical 0x hex requires {N} lowercase bytes")) };
    if hex.len() != N * 2 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) {
        return Err(format!("canonical 0x hex requires {N} lowercase bytes"));
    }
    let mut bytes = [0u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).map_err(|error| error.to_string())?;
    }
    Ok(bytes)
}
fn write_hex<const N: usize>(bytes: &[u8; N]) -> String { format!("0x{}", hex::encode(bytes)) }

impl TryFrom<String> for Bytes32Transport { type Error = String; fn try_from(value: String) -> Result<Self, Self::Error> { Ok(Self(fixed_hex(value)?)) } }
impl From<Bytes32Transport> for String { fn from(value: Bytes32Transport) -> Self { write_hex(&value.0) } }
impl TryFrom<String> for Bytes33Transport { type Error = String; fn try_from(value: String) -> Result<Self, Self::Error> { Ok(Self(fixed_hex(value)?)) } }
impl From<Bytes33Transport> for String { fn from(value: Bytes33Transport) -> Self { write_hex(&value.0) } }
impl TryFrom<String> for Bytes64Transport { type Error = String; fn try_from(value: String) -> Result<Self, Self::Error> { Ok(Self(fixed_hex(value)?)) } }
impl From<Bytes64Transport> for String { fn from(value: Bytes64Transport) -> Self { write_hex(&value.0) } }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hash_json() -> serde_json::Value { json!(["1", "2", "3", "4"]) }
    fn bytes_json(width: usize) -> String { format!("0x{}", "ab".repeat(width)) }

    #[test]
    fn noncanonical_hash_and_hex_are_rejected() {
        assert!(serde_json::from_value::<Hash4Transport>(json!(["18446744069414584321", "0", "0", "0"])).is_err());
        assert!(serde_json::from_value::<Hash4Transport>(json!(["01", "0", "0", "0"])).is_err());
        assert!(serde_json::from_value::<Hash4Transport>(json!(["18446744073709551616", "0", "0", "0"])).is_err());
        assert!(serde_json::from_value::<Bytes32Transport>(json!(format!("0x{}", "AB".repeat(32)))).is_err());
        assert!(serde_json::from_value::<Bytes32Transport>(json!("ab".repeat(32))).is_err());
        assert!(serde_json::from_value::<Bytes33Transport>(json!("0x02")).is_err());
        assert!(serde_json::from_value::<Bytes64Transport>(json!(format!("0x{}", "zz".repeat(64)))).is_err());
    }

    #[test]
    fn job_height_and_index_are_rejected() {
        for (height, path_index) in [(1u8, 0u32), (22, 0), (3, 2)] {
            let job = job_json(height, path_index);
            let Err(error) = serde_json::from_value::<RewardSessionJobTransport>(job).unwrap().decode() else { panic!("job accepted") };
            assert!(error.to_string().contains("reward session job"), "{error}");
        }
    }
    #[test]
    fn unknown_nested_leaf_and_policy_fields_are_rejected() {
        let witness = witness_json();
        assert!(serde_json::from_value::<RewardSessionWitnessTransport>(witness.clone()).is_ok());
        let mut witness = witness;
        witness["userLeaf"]["extra"] = json!(1);
        assert!(serde_json::from_value::<RewardSessionWitnessTransport>(witness).is_err());
        let authorization = multisig_json();
        assert!(serde_json::from_value::<RewardAuthorizationTransport>(authorization.clone()).unwrap().decode().is_ok());
        let mut authorization = authorization;
        authorization["initialPolicy"]["extra"] = json!(1);
        assert!(serde_json::from_value::<RewardAuthorizationTransport>(authorization).is_err());
    }


    #[test]
    fn old_claim_body_and_unknown_authorization_are_rejected() {
        let old = json!({
            "config": "", "registry": "", "servicesUrl": "", "context": {},
            "record": {}, "userId": "1", "job": {},
        });
        assert!(serde_json::from_value::<RewardSessionClaimProvingRequest>(old).is_err());
        assert!(serde_json::from_value::<RewardAuthorizationTransport>(json!({"scheme": "ticket"})).is_err());
    }

    #[test]
    fn statement_recipient_width_is_rejected() {
        let mut recipient = [0u32; 8];
        recipient[7] = 1;
        let fields = RewardSessionProofFieldsTransport {
            checkpoint_tree_root: Hash4Transport([0; 4]), user_id: 0, recipient, total_amount: [0; 8],
            count: 0, jobs_commitment: Hash4Transport([0; 4]), old_ledger_state_root: Hash4Transport([0; 4]),
            new_ledger_state_root: Hash4Transport([0; 4]),
        };
        assert!(fields.decode().unwrap_err().to_string().contains("recipient width"));
    }

    #[test]
    fn multisig_order_contract_and_path_height_are_rejected() {
        let policy = MultisigPolicy {
            version: 1, threshold: 2, member_count: 3,
            member_hashes: [1u64, 2, 3, 0, 0, 0, 0, 0].map(|value| ClientHash::from_values(value, 0, 0, 0)),
        };
        let members = serde_json::to_value(&policy.member_hashes).unwrap();
        let base = json!({
            "scheme": "multisig", "contractId": 6,
            "initialPolicy": { "version": 1, "threshold": 2, "member_count": 3, "member_hashes": members },
            "policySlots": [["1", "2", "3", "0"], ["1", "0", "0", "0"], ["2", "0", "0", "0"], ["0", "0", "0", "0"]],
            "contractStatePaths": [vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize]],
            "policySlotPaths": [vec![hash_json(); 4], vec![hash_json(); 4], vec![hash_json(); 4], vec![hash_json(); 4]],
            "memberIndices": [0, 1],
            "compressedPublicKeys": [bytes_json(33), bytes_json(33)],
            "signaturesRs": [bytes_json(64), bytes_json(64)],
        });
        assert!(serde_json::from_value::<RewardAuthorizationTransport>(base.clone()).unwrap().decode().is_ok());
        let mut reversed = base.clone();
        reversed["memberIndices"] = json!([1, 0]);
        assert!(serde_json::from_value::<RewardAuthorizationTransport>(reversed).unwrap().decode().is_err());
        let mut contract = base.clone();
        contract["contractId"] = json!(7);
        let Err(error) = serde_json::from_value::<RewardAuthorizationTransport>(contract).unwrap().decode() else { panic!("contract accepted") };
        assert!(error.to_string().contains("policy precompile"));
        let mut short = base;
        short["contractStatePaths"][0] = json!([]);
        let Err(error) = serde_json::from_value::<RewardAuthorizationTransport>(short).unwrap().decode() else { panic!("path accepted") };
        assert!(error.to_string().contains("path height"));
    }

    #[test]
    fn final_step_user_path_and_noncanonical_proof_are_rejected() {
        let mut witness = witness_json();
        witness["isFinalStep"] = json!(true);
        witness["authorization"] = json!(null);
        let Err(error) = serde_json::from_value::<RewardSessionWitnessTransport>(witness).unwrap().decode(&reward_session_common()) else { panic!("final step accepted") };
        assert!(error.to_string().contains("final-step authorization missing"));

        let mut witness = witness_json();
        witness["userPath"] = json!([]);
        let Err(error) = serde_json::from_value::<RewardSessionWitnessTransport>(witness).unwrap().decode(&reward_session_common()) else { panic!("user path accepted") };
        assert!(error.to_string().contains("user path length"));

        let mut witness = witness_json();
        witness["ownPrevious"] = json!(STANDARD.encode([1u8, 2, 3]));
        let Err(error) = serde_json::from_value::<RewardSessionWitnessTransport>(witness).unwrap().decode(&reward_session_common()) else { panic!("proof accepted") };
        assert!(error.to_string().contains("predecessor decoding"));

    }
    fn multisig_json() -> serde_json::Value {
        let policy = MultisigPolicy {
            version: 1, threshold: 2, member_count: 3,
            member_hashes: [1u64, 2, 3, 0, 0, 0, 0, 0].map(|value| ClientHash::from_values(value, 0, 0, 0)),
        };
        json!({
            "scheme": "multisig", "contractId": 6,
            "initialPolicy": { "version": 1, "threshold": 2, "member_count": 3, "member_hashes": serde_json::to_value(&policy.member_hashes).unwrap() },
            "policySlots": [["1", "2", "3", "0"], ["1", "0", "0", "0"], ["2", "0", "0", "0"], ["0", "0", "0", "0"]],
            "contractStatePaths": [vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize], vec![hash_json(); GLOBAL_CONTRACT_TREE_HEIGHT as usize]],
            "policySlotPaths": [vec![hash_json(); 4], vec![hash_json(); 4], vec![hash_json(); 4], vec![hash_json(); 4]],
            "memberIndices": [0, 1],
            "compressedPublicKeys": [bytes_json(33), bytes_json(33)],
            "signaturesRs": [bytes_json(64), bytes_json(64)],
        })
    }


    fn job_json(height: u8, path_index: u32) -> serde_json::Value {
        json!({
            "height": height, "pathIndex": path_index,
            "tag": { "tagPreimage": hash_json(), "leafLeft": hash_json(), "leafRight": hash_json(), "leafTag": hash_json(),
                "siblings": vec![hash_json(); REWARD_TAG_HEIGHT], "parentTags": vec![hash_json(); REWARD_TAG_HEIGHT] },
            "nullifierSiblings": vec![hash_json(); 63],
        })
    }

    fn witness_json() -> serde_json::Value {
        let leaf = serde_json::to_value(PsyCheckpointLeaf::<F>::default()).unwrap();
        let roots = serde_json::to_value(PsyCheckpointGlobalStateRoots::<F>::default()).unwrap();
        let user = serde_json::to_value(PsyUserLeaf::<F>::default()).unwrap();
        json!({
            "statement": { "checkpointTreeRoot": hash_json(), "userId": 0, "recipient": vec![0; 8], "totalAmount": vec![0; 8],
                "count": 0, "jobsCommitment": hash_json(), "oldLedgerStateRoot": hash_json(), "newLedgerStateRoot": hash_json() },
            "config": STANDARD.encode(valid_config().encode().unwrap()),
            "economicDomain": bytes_json(32), "windowId": bytes_json(32), "startRoot": hash_json(),
            "sourceCheckpointId": 0, "endCheckpointId": 0,
            "sourceLeaf": leaf,
            "sourcePath": vec![hash_json(); CHECKPOINT_TREE_HEIGHT as usize],
            "oldState": state_json(), "newState": state_json(), "ownState": state_json(),
            "oldSummary": hash_json(), "oldSessionRoot": hash_json(),
            "sessionSiblings": vec![hash_json(); 32], "ownSiblings": vec![hash_json(); 32], "ledgerSiblings": vec![hash_json(); 64],
            "ownPrevious": null, "globalPrevious": null, "jobs": [], "isFinalStep": false,
            "endLeaf": leaf,
            "endPath": vec![hash_json(); CHECKPOINT_TREE_HEIGHT as usize],
            "endRoots": roots,
            "userLeaf": user,
            "userPath": vec![hash_json(); GLOBAL_USER_TREE_HEIGHT as usize],
            "publicKeyParam": hash_json(), "authorization": null,
        })
    }
    fn valid_config() -> NetworkConfig {
        use psy_data::bridge_aggregate::{ChainConfig, BRIDGE_USER_ID};
        NetworkConfig {
            version: 1, network_magic: 1, bridge_user_id: BRIDGE_USER_ID, circuit_set_hash: [1; 32],
            chains: vec![ChainConfig { chain_index: 1, chain_id: [2; 32], bridge: [3; 20], state_manager: [4; 20], bootstrap_id: 1, bootstrap_root: [1, 0, 0, 0] }],
            ethereum_index: 1, reward_payer: [5; 20], reward_token: [6; 20], reward_per_claim: [1; 32],
            reward_token_decimals: 6, reward_cutover: 1, reward_end_exclusive: 2, max_deposits: 1, max_withdrawals: 1, max_rewards: 1,
        }
    }

    fn state_json() -> serde_json::Value {
        json!({ "ledgerWindowHash": hash_json(), "ledgerRoot": hash_json(), "userRoot": hash_json(), "sessionCount": 0, "unfinishedSessionCount": 0 })
    }

    fn reward_session_common() -> CommonCircuitData<F, 2> {
        use plonky2::plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitConfig};
        let mut builder = CircuitBuilder::<F, 2>::new(CircuitConfig::standard_recursion_config());
        for _ in 0..REWARD_SESSION_PROOF_FIELD_COUNT { builder.add_virtual_public_input(); }
        builder.build::<C>().common
    }

}
