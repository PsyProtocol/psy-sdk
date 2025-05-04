use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::merkle::core::DeltaMerkleProofCore;
use qed_data::{guta::api::QEDContractStateUpdateHistory, qdata::user::QEDUserLeaf};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub struct QEDLocalStateSetKey {
    pub contract: u64,
    pub slot: u64,
}

impl QEDLocalStateSetKey {
    pub fn new(contract: u64, slot: u64) -> Self {
        Self { contract, slot }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDLocalStateSet<F: RichField> {
    pub index: u64,
    pub start_value: QHashOut<F>,
    pub end_value: QHashOut<F>,
}

impl<F: RichField> QEDLocalStateSet<F> {
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
pub struct QEDUserSessionUpdateHistory<F: RichField> {
    pub start_user_leaf: QEDUserLeaf<F>,
    pub end_user_leaf: QEDUserLeaf<F>,
    pub total_slots_modified: u32,
    pub contract_updates: Vec<QEDContractStateUpdateHistory<F>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDStateTrackerContractResult<F: RichField> {
    pub contract_id: u64,
    pub slots: Vec<QEDLocalStateSet<F>>,
    pub total_slots_modified: u32,
    pub start_state_root: QHashOut<F>,
    pub end_state_root: QHashOut<F>,
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDContractStateTracker<F: RichField> {
    pub contract_id: u64,
    pub slots: hashbrown::HashMap<u64, QEDLocalStateSet<F>>,
    pub total_slots_modified: u32,
    pub start_state_root: QHashOut<F>,
    pub end_state_root: QHashOut<F>,
}

impl<F: RichField> QEDContractStateTracker<F> {
    pub fn new(contract_id: u64) -> Self {
        Self {
            contract_id,
            slots: hashbrown::HashMap::new(),
            total_slots_modified: 0,
            start_state_root: QHashOut::ZERO,
            end_state_root: QHashOut::ZERO,
        }
    }
    #[instrument(skip(self, dmp), fields(contract_id = self.contract_id, slot_index = dmp.index, total_slots_modified = self.total_slots_modified))]
    pub fn notify_update_slot_dmp(&mut self, dmp: &DeltaMerkleProofCore<QHashOut<F>>) -> i32 {
        eprintln!("DEBUGPRINT[587]: state_tracker.rs:88: dmp={}", serde_json::to_string_pretty(&dmp).unwrap());
        if self.total_slots_modified == 0 {
            self.start_state_root = dmp.old_root;
            self.end_state_root = dmp.new_root;
            self.slots.insert(
                dmp.index,
                QEDLocalStateSet {
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
                        QEDLocalStateSet {
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
    pub fn to_result(&self) -> QEDStateTrackerContractResult<F> {
        QEDStateTrackerContractResult {
            contract_id: self.contract_id,
            slots: self.slots.values().map(|x|x.to_owned()).collect::<Vec<_>>(),
            total_slots_modified: self.total_slots_modified,
            start_state_root: self.start_state_root,
            end_state_root: self.end_state_root,
        }
    }
}

#[derive(Clone, Debug)]
pub struct QEDLocalStateTracker<F: RichField> {
    pub contracts: hashbrown::HashMap<u64, QEDContractStateTracker<F>>,
    pub total_slots_modified: u32,
}

impl<F: RichField> QEDLocalStateTracker<F> {
    pub fn new() -> Self {
        Self {
            contracts: hashbrown::HashMap::new(),
            total_slots_modified: 0,
        }
    }
    pub fn notify_update_slot_dmp(
        &mut self,
        contract_id: u64,
        dmp: &DeltaMerkleProofCore<QHashOut<F>>,
    ) {
        let inc_modified_slots = match self.contracts.get_mut(&contract_id) {
            Some(c) => c.notify_update_slot_dmp(dmp),
            None => {
                let mut tracker = QEDContractStateTracker::new(contract_id);
                let result = tracker.notify_update_slot_dmp(dmp);
                self.contracts.insert(contract_id, tracker);
                result
            }
        };

        self.total_slots_modified = ((self.total_slots_modified as i32)+inc_modified_slots) as u32;
    }
    pub fn get_results(&self) -> Vec<QEDStateTrackerContractResult<F>> {
        self.contracts.values().map(|x|x.to_result()).collect()
    }
}
