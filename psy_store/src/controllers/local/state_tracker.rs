use indexmap::IndexMap;
use plonky2::hash::hash_types::RichField;
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::merkle::core::DeltaMerkleProofCore;
use psy_data::{guta::api::PsyContractStateUpdateHistory, qdata::user::PsyUserLeaf};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub struct PsyLocalStateSetKey {
    pub contract: u64,
    pub slot: u64,
}

impl PsyLocalStateSetKey {
    pub fn new(contract: u64, slot: u64) -> Self {
        Self { contract, slot }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PsyLocalStateSet<F: RichField> {
    pub index: u64,
    pub start_value: QHashOut<F>,
    pub end_value: QHashOut<F>,
}

impl<F: RichField> PsyLocalStateSet<F> {
    pub fn new(index: u64, start_value: QHashOut<F>, end_value: QHashOut<F>) -> Self {
        Self {
            index,
            start_value,
            end_value,
        }
    }
    pub fn from_dmp(dmp: &DeltaMerkleProofCore<QHashOut<F>>) -> Self {
        Self {
            index: dmp.index,
            start_value: dmp.old_value,
            end_value: dmp.new_value,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PsyUserSessionUpdateHistory<F: RichField> {
    pub start_user_leaf: PsyUserLeaf<F>,
    pub end_user_leaf: PsyUserLeaf<F>,
    pub total_slots_modified: u32,
    pub contract_updates: Vec<PsyContractStateUpdateHistory<F>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PsyStateTrackerContractResult<F: RichField> {
    pub contract_id: u64,
    pub slots: Vec<PsyLocalStateSet<F>>,
    pub total_slots_modified: u32,
    pub start_state_root: QHashOut<F>,
    pub end_state_root: QHashOut<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct PsyContractStateTracker<F: RichField> {
    pub contract_id: u64,
    pub slots: IndexMap<u64, PsyLocalStateSet<F>>,
    pub total_slots_modified: u32,
    pub start_state_root: QHashOut<F>,
    pub end_state_root: QHashOut<F>,
}

impl<F: RichField> PsyContractStateTracker<F> {
    pub fn new(contract_id: u64) -> Self {
        Self {
            contract_id,
            slots: IndexMap::new(),
            total_slots_modified: 0,
            start_state_root: QHashOut::ZERO,
            end_state_root: QHashOut::ZERO,
        }
    }
    #[instrument(skip(self, dmp), fields(contract_id = self.contract_id, slot_index = dmp.index, total_slots_modified = self.total_slots_modified))]
    pub fn notify_update_slot_dmp(&mut self, dmp: &DeltaMerkleProofCore<QHashOut<F>>) -> i32 {
        tracing::debug!("State tracker DMP: {}", serde_json::to_string_pretty(&dmp).unwrap());
        if self.total_slots_modified == 0 {
            self.start_state_root = dmp.old_root;
            self.end_state_root = dmp.new_root;
            self.slots.insert(
                dmp.index,
                PsyLocalStateSet {
                    index: dmp.index,
                    start_value: dmp.old_value,
                    end_value: dmp.new_value,
                },
            );
            self.total_slots_modified = 1;
            1
        } else {
            let inc = match self.slots.get_mut(&dmp.index) {
                Some(v) => {
                    v.end_value = dmp.new_value;
                    if v.end_value.eq(&v.start_value) {
                        -1
                    } else {
                        0
                    }
                }
                None => {
                    self.slots.insert(
                        dmp.index,
                        PsyLocalStateSet {
                            index: dmp.index,
                            start_value: dmp.old_value,
                            end_value: dmp.new_value,
                        },
                    );
                    1
                }
            };
            if inc == -1 {
                self.slots.remove(&dmp.index);
            }

            self.total_slots_modified = ((self.total_slots_modified as i32) + inc) as u32;
            self.end_state_root = dmp.new_root;
            inc
        }
    }

    pub fn to_result(&self) -> PsyStateTrackerContractResult<F> {
        PsyStateTrackerContractResult {
            contract_id: self.contract_id,
            slots: self.slots.values().map(|x| x.to_owned()).collect::<Vec<_>>(),
            total_slots_modified: self.total_slots_modified,
            start_state_root: self.start_state_root,
            end_state_root: self.end_state_root,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PsyLocalStateTracker<F: RichField> {
    pub contracts: IndexMap<u64, PsyContractStateTracker<F>>,
    pub total_slots_modified: u32,
}

impl<F: RichField> PsyLocalStateTracker<F> {
    pub fn new() -> Self {
        Self {
            contracts: IndexMap::new(),
            total_slots_modified: 0,
        }
    }
    pub fn notify_update_slot_dmp(&mut self, contract_id: u64, dmp: &DeltaMerkleProofCore<QHashOut<F>>) {
        let inc_modified_slots = match self.contracts.get_mut(&contract_id) {
            Some(c) => c.notify_update_slot_dmp(dmp),
            None => {
                let mut tracker = PsyContractStateTracker::new(contract_id);
                let result = tracker.notify_update_slot_dmp(dmp);
                self.contracts.insert(contract_id, tracker);
                result
            }
        };

        self.total_slots_modified = ((self.total_slots_modified as i32) + inc_modified_slots) as u32;
    }

    pub fn get_results(&self) -> Vec<PsyStateTrackerContractResult<F>> {
        self.contracts.values().map(|x| x.to_result()).collect()
    }

    pub fn get_contract_result(&self, contract_id: u64) -> Option<PsyStateTrackerContractResult<F>> {
        self.contracts.get(&contract_id).map(|c| c.to_result())
    }
}
