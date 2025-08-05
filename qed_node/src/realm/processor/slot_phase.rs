use crate::common::slot::{Clock, Slot, SLOT_SIZE};

#[derive(Debug, Clone, PartialEq)]
pub enum SlotPhase {
    EarlyPhase(u64),
    BuildPhase(u64),
    ProofPhase(u64),
    SubmitPhase(u64),
}

impl SlotPhase {
    pub fn get_build_phase(slot: impl Slot) -> SlotPhase {
        let slot_phase_config: SlotPhaseConfig = SlotPhaseConfig::default();
        SlotPhase::BuildPhase(
            slot.get_current_slot_timestamp()
                + (slot_phase_config.early_phase_end_percent * SLOT_SIZE as f64) as u64,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SlotPhaseConfig {
    pub early_phase_end_percent: f64,  // 40%
    pub build_phase_end_percent: f64,  // 10%
    pub proof_phase_end_percent: f64,  // 40%
    pub submit_phase_end_percent: f64, // 10%
}

impl Default for SlotPhaseConfig {
    fn default() -> Self {
        Self {
            early_phase_end_percent: 0.4,
            build_phase_end_percent: 0.10,
            proof_phase_end_percent: 0.40,
            submit_phase_end_percent: 0.1,
        }
    }
}

impl<T: Slot> From<T> for SlotPhase {
    fn from(slot: T) -> Self {
        let slot_phase_config = SlotPhaseConfig::default();
        let slot_start_timestamp = slot.get_current_slot_timestamp();
        let current_time = slot.get_current_timestamp();

        let early_phase_start_timestamp = slot_start_timestamp;
        let build_phase_start_timestamp = slot_start_timestamp
            + (slot_phase_config.early_phase_end_percent * SLOT_SIZE as f64) as u64;
        let proof_phase_start_timestamp = slot_start_timestamp
            + ((slot_phase_config.early_phase_end_percent
                + slot_phase_config.build_phase_end_percent)
                * SLOT_SIZE as f64) as u64;
        let submit_phase_start_timestamp = slot_start_timestamp
            + ((slot_phase_config.early_phase_end_percent
                + slot_phase_config.build_phase_end_percent
                + slot_phase_config.proof_phase_end_percent)
                * SLOT_SIZE as f64) as u64;

        if current_time >= early_phase_start_timestamp && current_time < build_phase_start_timestamp {
            return SlotPhase::EarlyPhase(early_phase_start_timestamp);
        }

        if current_time >= build_phase_start_timestamp && current_time < proof_phase_start_timestamp {
            return SlotPhase::BuildPhase(build_phase_start_timestamp);
        }

        if current_time >= proof_phase_start_timestamp && current_time < submit_phase_start_timestamp {
            return SlotPhase::ProofPhase(proof_phase_start_timestamp);
        }

        SlotPhase::SubmitPhase(submit_phase_start_timestamp)
    }
}
