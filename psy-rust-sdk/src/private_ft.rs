//! Client adapter for versioned single-Felt and two-limb private FT packets.
//! No runtime or circuit changes. All integers remain exact throughout.
use base64::Engine;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField as F,
        types::{Field, PrimeField64},
    },
    hash::poseidon::PoseidonHash,
    plonk::{
        config::{Hasher, PoseidonGoldilocksConfig as C},
        proof::ProofWithPublicInputs,
    },
};
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use psy_crypto::hash::merkle::core::MerkleProofCore;
use psy_dpn_circuit::circuits::privacy::private_note_inclusion::PrivateNoteInclusionCircuit;
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, String>;
pub const P: u64 = 18446744069414584321;
pub const B: u64 = 1 << 62;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    SingleFeltNativeV1,
    SingleFeltPsy20V1,
    U124PrivateV1,
}
impl Profile {
    pub fn wide(self) -> bool {
        self == Self::U124PrivateV1
    }
    pub fn roots(self) -> [u64; 2] {
        match self {
            Self::SingleFeltNativeV1 => [2147483649, 0],
            Self::SingleFeltPsy20V1 => [8388609, 0],
            Self::U124PrivateV1 => [293601306, 293601307],
        }
    }
    pub fn fields_slots(self) -> (u64, u64) {
        match self {
            Self::SingleFeltNativeV1 => (0, 0),
            Self::SingleFeltPsy20V1 => (22, 2149832488),
            Self::U124PrivateV1 => (24, 2178599700),
        }
    }
    pub fn kind(self) -> &'static str {
        if self.wide() {
            "psy_private_ft_u124_v1"
        } else {
            "psy_private_ft_single_v1"
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packet {
    pub kind: String,
    pub contract_id: u64,
    pub owner: [u64; 4],
    pub amount: [u64; 2],
    pub checkpoint: u64,
    pub user_tree_root: [u64; 4],
    pub nullifiers: [[u64; 4]; 2],
    pub proofs: [Option<String>; 2],
}
pub fn words(h: QHashOut<F>) -> [u64; 4] {
    h.0.elements.map(|e| e.to_canonical_u64())
}
pub fn hash(v: &[u64]) -> QHashOut<F> {
    QHashOut(PoseidonHash::hash_no_pad(
        &v.iter()
            .map(|x| F::from_canonical_u64(*x))
            .collect::<Vec<_>>(),
    ))
}
pub fn hh(a: QHashOut<F>, b: QHashOut<F>) -> QHashOut<F> {
    QHashOut(PoseidonHash::two_to_one(a.0, b.0))
}
pub fn amount(profile: Profile, p: &Packet) -> Result<u128> {
    if p.kind != profile.kind()
        || p.contract_id >= 1 << 24
        || p.checkpoint == 0
        || p.checkpoint >= P
    {
        return Err("Invalid FT packet identity".into());
    }
    if (profile == Profile::SingleFeltNativeV1) != matches!(p.contract_id, 0 | 4) {
        return Err("FT profile/contract mismatch".into());
    }
    let n = if profile.wide() {
        if p.amount.iter().any(|v| *v >= B) {
            return Err("Noncanonical U124 limbs".into());
        }
        p.amount[0] as u128 + (p.amount[1] as u128) * (B as u128)
    } else {
        if p.amount[0] >= P || p.amount[1] != 0 {
            return Err("Noncanonical single-Felt amount".into());
        }
        p.amount[0] as u128
    };
    if n == 0
        || p.owner
            .iter()
            .chain(&p.user_tree_root)
            .chain(p.nullifiers.iter().flatten())
            .any(|v| *v >= P)
    {
        return Err("Noncanonical FT packet".into());
    }
    Ok(n)
}
#[cfg(test)]
pub fn circuit() -> Result<PrivateNoteInclusionCircuit<C, 2>> {
    let c = PrivateNoteInclusionCircuit::new(32, 24, 32, 20);
    if words(c.get_fingerprint())
        != [
            2482434993266711942,
            6028864160300535936,
            2827549014984201026,
            3018365102481798071,
        ]
    {
        return Err("Unsupported native note verifier".into());
    }
    Ok(c)
}
pub fn verify_with_circuit(
    profile: Profile,
    p: &Packet,
    c: &PrivateNoteInclusionCircuit<C, 2>,
) -> Result<Vec<(usize, ProofWithPublicInputs<F, C, 2>)>> {
    amount(profile, p)?;
    if words(c.get_fingerprint())
        != [
            2482434993266711942,
            6028864160300535936,
            2827549014984201026,
            3018365102481798071,
        ]
    {
        return Err("Unsupported native note verifier".into());
    }
    if p.amount.iter().all(|v| *v > 0) && p.nullifiers[0] == p.nullifiers[1] {
        return Err("Duplicate FT denomination nullifier".into());
    }
    let mut out = vec![];
    for i in 0..2 {
        if p.amount[i] == 0 {
            if p.proofs[i].is_some() || p.nullifiers[i] != [0; 4] {
                return Err("Zero denomination carries a proof".into());
            }
            continue;
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(p.proofs[i].as_deref().ok_or("Missing FT proof")?)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("FT proof too large".into());
        }
        use bincode::Options;
        let proof: ProofWithPublicInputs<F, C, 2> = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_limit(1024 * 1024)
            .reject_trailing_bytes()
            .deserialize(&bytes)
            .map_err(|e| e.to_string())?;
        let mut inputs = p.owner.to_vec();
        inputs.push(p.amount[i]);
        inputs.extend(p.user_tree_root);
        inputs.extend([p.checkpoint, profile.roots()[i], p.contract_id]);
        inputs.extend(p.nullifiers[i]);
        if proof.public_inputs != hash(&inputs).0.elements {
            return Err("FT proof binding mismatch".into());
        }
        plonky2::plonk::circuit_data::VerifierCircuitData {
            verifier_only: c.get_verifier_config_ref().clone(),
            common: c.get_common_circuit_data_ref().clone(),
        }
        .verify(proof.clone())
        .map_err(|e| e.to_string())?;
        out.push((i, proof));
    }
    Ok(out)
}

/// Refresh an earlier path after a later append using the later insertion's
/// authenticated path. Check both resulting roots; never reuse stale siblings.
pub fn refresh_path(
    earlier: &MerkleProofCore<QHashOut<F>>,
    later: &MerkleProofCore<QHashOut<F>>,
) -> Result<MerkleProofCore<QHashOut<F>>> {
    if earlier.siblings.len() != 16
        || later.siblings.len() != 16
        || earlier.index >= later.index
        || later.index >= 65536
    {
        return Err("Invalid FT proof-tree insertion order".into());
    }
    let level = (63 - (earlier.index ^ later.index).leading_zeros()) as usize;
    let mut subtree = later.value;
    for k in 0..level {
        subtree = if (later.index >> k) & 1 == 0 {
            hh(subtree, later.siblings[k])
        } else {
            hh(later.siblings[k], subtree)
        };
    }
    let mut siblings = earlier.siblings.clone();
    siblings[level] = subtree;
    let path = MerkleProofCore::new_from_params::<psy_crypto::hash::traits::hasher::PoseidonHasher>(
        earlier.index,
        earlier.value,
        siblings,
    );
    if path.root != later.root {
        return Err("FT refreshed proof path root mismatch".into());
    }
    Ok(path)
}
pub fn claim_inputs(
    profile: Profile,
    p: &Packet,
    r0: u64,
    r1: u64,
    paths: &[Vec<u64>; 2],
) -> Result<Vec<u64>> {
    amount(profile, p)?;
    if r0 >= P
        || r1 >= P
        || paths
            .iter()
            .any(|p| p.len() != 65 || p.iter().any(|v| *v >= P))
    {
        return Err("Invalid FT claim inputs".into());
    }
    let mut inputs = vec![];
    if profile.wide() {
        inputs.extend(p.owner);
        inputs.extend(p.amount);
        inputs.extend(p.user_tree_root);
        inputs.extend([p.checkpoint, r0, r1]);
        inputs.extend(p.nullifiers[0]);
        inputs.extend(p.nullifiers[1]);
        inputs.extend(&paths[0]);
        inputs.extend(&paths[1]);
    } else {
        inputs.extend(p.nullifiers[0]);
        inputs.extend(p.owner);
        inputs.push(p.amount[0]);
        inputs.extend(p.user_tree_root);
        inputs.extend([p.checkpoint, profile.roots()[0], r0, r1]);
        inputs.extend(&paths[0]);
    }
    Ok(inputs)
}
#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::hash::hash_types::HashOut;
    #[test]
    fn claim_width_and_amount_boundaries() {
        let mut p = Packet {
            kind: Profile::U124PrivateV1.kind().into(),
            contract_id: 2457,
            owner: [1; 4],
            amount: [B - 1, B - 1],
            checkpoint: 1,
            user_tree_root: [2; 4],
            nullifiers: [[3; 4]; 2],
            proofs: [None, None],
        };
        assert_eq!(
            amount(Profile::U124PrivateV1, &p).unwrap(),
            (1u128 << 124) - 1
        );
        assert_eq!(
            claim_inputs(
                Profile::U124PrivateV1,
                &p,
                1,
                2,
                &[vec![0; 65], vec![0; 65]]
            )
            .unwrap()
            .len(),
            151
        );
        assert!(amount(Profile::SingleFeltNativeV1, &p).is_err());
        p.kind = Profile::SingleFeltNativeV1.kind().into();
        p.contract_id = 0;
        p.amount = [P - 1, 0];
        assert_eq!(
            claim_inputs(
                Profile::SingleFeltNativeV1,
                &p,
                1,
                2,
                &[vec![0; 65], vec![0; 65]]
            )
            .unwrap()
            .len(),
            82
        );
    }
    #[test]
    fn native_proofs_bind_both_denominations_and_reject_relabeling() {
        use psy_crypto::hash::traits::{hasher::PoseidonHasher, qhashable::QFieldHashable};
        use psy_data::{
            privacy::private_note_inclusion::PrivateNoteInclusionInput, qdata::user::PsyUserLeaf,
        };
        let c = circuit().unwrap();
        let zero = QHashOut(HashOut::ZERO);
        let proof_path = |index, value, siblings| {
            MerkleProofCore::new_from_params::<PoseidonHasher>(index, value, siblings)
        };
        for (profile, cid, amounts) in [
            (Profile::SingleFeltNativeV1, 0, [P - 1, 0]),
            (Profile::SingleFeltPsy20V1, 53, [B + 1, 0]),
            (Profile::U124PrivateV1, 2457, [17, 2]),
        ] {
            let owner = hash(&[1, 2, 3, 4]);
            let secrets = [hash(&[5, 6]), hash(&[7, 8])];
            let nullifier_secrets = [hash(&[9, 10]), hash(&[11, 12])];
            let mut memberships = vec![];
            for i in 0..2 {
                let mut tuple = words(nullifier_secrets[i]).to_vec();
                tuple.extend(words(secrets[i]));
                let commit = hh(
                    hh(owner, QHashOut::from_values(amounts[i], 0, 0, 0)),
                    hash(&tuple),
                );
                memberships.push(proof_path(0, commit, vec![zero; 20]));
            }
            let mut slots = vec![];
            for i in 0..if profile.wide() { 2 } else { 1 } {
                let mut siblings = vec![zero; 32];
                if profile.wide() {
                    siblings[0] = memberships[1 - i].root;
                }
                slots.push(proof_path(
                    profile.roots()[i],
                    memberships[i].root,
                    siblings,
                ));
            }
            let contract = proof_path(cid, slots[0].root, vec![zero; 24]);
            let leaf = PsyUserLeaf::new_user_default(F::ONE, hash(&[44]), contract.root);
            let user = proof_path(1, leaf.qfhash::<PoseidonHasher>(), vec![zero; 32]);
            let mut packet = Packet {
                kind: profile.kind().into(),
                contract_id: cid,
                owner: words(owner),
                amount: amounts,
                checkpoint: 123,
                user_tree_root: words(user.root),
                nullifiers: [[0; 4]; 2],
                proofs: [None, None],
            };
            for i in 0..if profile.wide() { 2 } else { 1 } {
                let input = PrivateNoteInclusionInput {
                    nullifier_secret: nullifier_secrets[i],
                    sender_user_id: 1,
                    contract_id: cid,
                    user_leaf: leaf.clone(),
                    owner,
                    amount: F::from_canonical_u64(amounts[i]),
                    note_secret: secrets[i],
                    note_membership_proof: memberships[i].clone(),
                    note_root_slot: profile.roots()[i],
                    note_root_slot_proof: slots[i].clone(),
                    contract_proof: contract.clone(),
                    user_tree_proof: user.clone(),
                    checkpoint_id: F::from_canonical_u64(123),
                };
                let proof = c.prove(&input).unwrap();
                packet.nullifiers[i] = words(hash(&words(nullifier_secrets[i])));
                packet.proofs[i] = Some(
                    base64::engine::general_purpose::STANDARD
                        .encode(bincode::serialize(&proof).unwrap()),
                );
            }
            assert_eq!(
                verify_with_circuit(profile, &packet, &c).unwrap().len(),
                if profile.wide() { 2 } else { 1 }
            );
            let mut bad = packet.clone();
            bad.contract_id += 1;
            assert!(verify_with_circuit(profile, &bad, &c).is_err());
            bad = packet.clone();
            bad.amount[0] -= 1;
            assert!(verify_with_circuit(profile, &bad, &c).is_err());
            bad = packet.clone();
            bad.user_tree_root[0] = 0;
            assert!(verify_with_circuit(profile, &bad, &c).is_err());
            if profile.wide() {
                bad = packet.clone();
                bad.proofs.swap(0, 1);
                assert!(verify_with_circuit(profile, &bad, &c).is_err());
            }
        }
    }
    #[test]
    fn path_refresh_changes_the_branch_at_each_possible_height() {
        // Build sparse trees at all 16 divergence heights. An append has to
        // replace exactly the sibling subtree that contains that new leaf.
        let zero = QHashOut(HashOut::ZERO);
        let first = hash(&[7]);
        let second = hash(&[8]);
        let mut zeros = vec![zero];
        for k in 0..16 {
            zeros.push(hh(zeros[k], zeros[k]));
        }
        let earlier = MerkleProofCore::new_from_params::<
            psy_crypto::hash::traits::hasher::PoseidonHasher,
        >(0, first, zeros[..16].to_vec());
        for level in 0..16 {
            let mut siblings = zeros[..16].to_vec();
            let mut branch = first;
            for k in 0..level {
                branch = hh(branch, zeros[k]);
            }
            siblings[level] = branch;
            let later = MerkleProofCore::new_from_params::<
                psy_crypto::hash::traits::hasher::PoseidonHasher,
            >(1 << level, second, siblings);
            let refreshed = refresh_path(&earlier, &later).unwrap();
            assert_eq!(refreshed.root, later.root);
            assert_ne!(refreshed.root, earlier.root);
        }
    }
}

pub async fn pin(
    p: &psy_provider::provider::RpcProvider,
    profile: Profile,
    cid: u64,
    code_root: &str,
    layout_root: &str,
) -> anyhow::Result<()> {
    use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
    let c = p.get_contract_leaf_data(cid).await?;
    let (fields, slots) = profile.fields_slots();
    anyhow::ensure!(
        c.code_root.to_string() == code_root
            && c.state_layout_root.to_string() == layout_root
            && c.state_tree_height.to_canonical_u64() == 32
            && c.state_layout_field_count.to_canonical_u64() == fields
            && c.state_layout_slot_count.to_canonical_u64() == slots,
        "Private FT chain code/layout/profile changed"
    );
    let code = p.get_contract_code_definition(cid).await?;
    anyhow::ensure!(code.state_tree_height == 32, "FT tree height mismatch");
    let defs = code
        .functions
        .iter()
        .map(psy_vm::dpn::contract::cfc_code_definition_to_dapen_fc)
        .collect::<anyhow::Result<Vec<_>>>()?;
    for (f, d) in code.functions.iter().zip(&defs) {
        anyhow::ensure!(
            f.vm_type == 1
                && f.method_id == d.method_id
                && f.num_inputs as usize == d.circuit_inputs.len()
                && f.num_outputs as usize == d.circuit_outputs.len(),
            "FT descriptor mismatch"
        );
    }
    let hashes = defs
        .iter()
        .map(psy_vm::dpn::contract::hash_dpn_function::<F>)
        .collect::<Vec<_>>();
    let root = psy_data::qblock::cmds::deploy_contract::get_code_root_by_code_hashes::<
        F,
        <C as plonky2::plonk::config::GenericConfig<2>>::Hasher,
    >(
        &hashes,
        psy_config::network_constants::CONTRACT_FUNCTION_TREE_HEIGHT - 1,
    );
    anyhow::ensure!(
        root == c.code_root,
        "FT methods do not authenticate to code root"
    );
    for (name, size) in [
        ("private_transfer", if profile.wide() { 14 } else { 9 }),
        ("private_claim", if profile.wide() { 151 } else { 82 }),
    ] {
        let matches = defs.iter().filter(|d| d.name == name).collect::<Vec<_>>();
        anyhow::ensure!(
            matches.len() == 1
                && matches[0].circuit_inputs.len() == size
                && matches[0].circuit_outputs.is_empty(),
            "FT method shape mismatch"
        );
    }
    Ok(())
}

pub async fn verify_checkpoint(
    provider: &psy_provider::provider::RpcProvider,
    packet: &Packet,
) -> anyhow::Result<()> {
    use psy_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
    anyhow::ensure!(
        words(provider.get_user_tree_root(packet.checkpoint).await?) == packet.user_tree_root,
        "Private FT user root does not match on-chain checkpoint"
    );
    Ok(())
}

/// Single-Felt contracts add in the field; reject wrap before proving/signing.
pub fn single_credit_capacity(balance: u64, credit: u64) -> Result<()> {
    if balance >= P || credit >= P || balance as u128 + credit as u128 >= P as u128 {
        return Err("Private FT claim would overflow its single-Felt balance".into());
    }
    Ok(())
}
#[cfg(test)]
#[test]
fn single_credit_rejects_boundary_and_cumulative_batch_wrap() {
    assert!(single_credit_capacity(P - 2, 1).is_ok());
    assert!(single_credit_capacity(P - 2, 2).is_err());
    assert!(single_credit_capacity(P - 2, 3).is_err());
    let start = P - 3;
    assert!(single_credit_capacity(start, 2).is_ok());
    assert!(single_credit_capacity(start + 2, 1).is_err());
}
