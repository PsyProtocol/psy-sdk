use chrono::Utc;
use jsonwebtoken::get_current_timestamp;

pub const SLOT_SIZE: u64 = 6000; // 6s
pub const SLOT0_TIMESTAMP: u64 = 1753891200000; // 2025-07-31 00:00:00
pub const NETWORK_COST_TIME_MS: u64 = 500; // 500ms


// pub fn get_slot_from_timestamp(timestamp: u64) -> u64 {
//     (timestamp - SLOT0_TIMESTAMP) / SLOT_SIZE
// }
//
// pub fn get_timestamp_from_slot(slot: u64) -> u64 {
//     SLOT0_TIMESTAMP + slot * SLOT_SIZE
// }
// pub fn get_current_slot() -> u64 {
//     get_slot_from_timestamp(Utc::now().timestamp_millis() as u64)
// }
//
// pub fn get_current_timestamp() -> u64 {
//     Utc::now().timestamp_millis() as u64
// }
//
// pub fn get_current_slot_timestamp() -> u64 {
//     get_timestamp_from_slot(get_current_slot())
// }
//
// // next slot timestamp
// pub fn get_next_slot_timestamp() -> u64 {
//     get_timestamp_from_slot(get_current_slot() + 1)
// }
//
// // retain time to next slot time stamp
// pub fn get_retain_time_to_next_slot() -> u64 {
//     get_next_slot_timestamp() - get_current_timestamp()
// }
// // current slot elapsed time
// pub fn get_current_slot_elapsed_time() -> u64 {
//     get_current_timestamp() - get_current_slot_timestamp()
// }
//
// // retain time to specific slot time stamp
// pub fn get_retain_time_to_slot(slot: u64) -> anyhow<u64> {
//     if slot < get_current_slot() {
//         return Err(anyhow::anyhow!("slot is too old"));
//     }
//     get_timestamp_from_slot(slot) - get_current_timestamp()
// }
//
// pub fn get_current_slot_time_to_next_slot() -> u64 {
//     get_next_slot_timestamp() - get_current_timestamp()
// }
//
// // is can reach to next slot
// pub fn is_can_reach_to_next_slot() -> bool {
//     get_current_slot_time_to_next_slot() <= NETWORK_COST_TIME_MS
// }
//
// pub fn is_can_reach_to_slot(slot: u64) -> bool {
//     get_retain_time_to_slot(slot) <= NETWORK_COST_TIME_MS
// }

pub trait Clock {
    fn get_current_timestamp() -> u64;
}

pub trait Slot {
    fn get_slot_from_timestamp(timestamp: u64) -> u64 {
        (timestamp - SLOT0_TIMESTAMP) / SLOT_SIZE
    }

    fn get_timestamp_from_slot(slot: u64) -> u64 {
        SLOT0_TIMESTAMP + slot * SLOT_SIZE
    }
    fn get_current_slot() -> u64 {
        Self::get_slot_from_timestamp(get_current_timestamp())
    }

    fn get_current_slot_timestamp() -> u64 {
        Self::get_timestamp_from_slot(Self::get_current_slot())
    }

    // next slot timestamp
    fn get_next_slot_timestamp() -> u64 {
        Self::get_timestamp_from_slot(Self::get_current_slot() + 1)
    }

    // retain time to next slot time stamp
    fn get_retain_time_to_next_slot() -> u64 {
        Self::get_next_slot_timestamp() - Self::get_current_timestamp()
    }
    // current slot elapsed time
    fn get_current_slot_elapsed_time() -> u64 {
        get_current_timestamp() - Self::get_current_slot_timestamp()
    }

    // retain time to specific slot time stamp
    fn get_retain_time_to_slot(slot: u64) -> anyhow::Result<u64> {
        if slot < Self::get_current_slot() {
            return Err(anyhow::anyhow!("slot is too old"));
        }
        Ok(Self::get_timestamp_from_slot(slot) - Self::get_current_timestamp())
    }

    fn get_current_slot_time_to_next_slot() -> u64 {
        Self::get_next_slot_timestamp() - Self::get_current_timestamp()
    }

    // is can reach to next slot
    fn is_can_reach_to_next_slot() -> bool {
        Self::get_current_slot_time_to_next_slot() <= NETWORK_COST_TIME_MS
    }

    fn is_can_reach_to_slot(slot: u64) -> anyhow::Result<bool> {
        Ok(Self::get_retain_time_to_slot(slot)? <= NETWORK_COST_TIME_MS)
    }
    fn get_current_timestamp() -> u64;
}

impl<T: Clock> Slot for T {
    fn get_current_timestamp() -> u64 {
        T::get_current_timestamp()
    }
}

pub struct LocalClock;

impl Clock for LocalClock {
    fn get_current_timestamp() -> u64 {
        Utc::now().timestamp_millis() as u64
    }
}
