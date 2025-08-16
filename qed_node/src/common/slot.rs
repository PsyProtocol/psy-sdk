use auto_impl::auto_impl;
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub const SLOT0: u64 = 0;
pub const SLOT_SIZE: u64 = 6000; // 6s
pub const SLOT0_TIMESTAMP: u64 = 1753891200000; // 2025-07-31 00:00:00
pub const NETWORK_COST_TIME_MS: u64 = 500; // 500ms

#[auto_impl(&, Box, Arc)]
pub trait Clock {
    fn get_current_timestamp(&self) -> u64;
}

// #[auto_impl(&, Box, Arc)]
pub trait Slot: Clock {
    /// Convert a timestamp to its corresponding slot number
    ///
    /// # Arguments
    /// * `timestamp` - The timestamp in milliseconds to convert
    ///
    /// # Returns
    /// The slot number corresponding to the given timestamp
    ///
    /// # Example
    /// ```
    /// let slot = clock.get_slot_from_timestamp(1753891200000); // Returns 0
    /// ```
    fn get_slot_from_timestamp(&self, timestamp: u64) -> u64 {
        (timestamp - SLOT0_TIMESTAMP) / SLOT_SIZE
    }

    /// Convert a slot number to its start timestamp
    ///
    /// # Arguments
    /// * `slot` - The slot number to convert
    ///
    /// # Returns
    /// The start timestamp (in milliseconds) of the specified slot
    ///
    /// # Example
    /// ```
    /// let timestamp = clock.get_timestamp_from_slot(1); // Returns SLOT0_TIMESTAMP + SLOT_SIZE
    /// ```
    fn get_timestamp_from_slot(&self, slot: u64) -> u64 {
        SLOT0_TIMESTAMP + slot * SLOT_SIZE
    }

    /// Get the current slot number based on the current timestamp
    ///
    /// # Returns
    /// The current slot number
    ///
    /// # Example
    /// ```
    /// let current_slot = clock.get_current_slot();
    /// ```
    fn get_current_slot(&self) -> u64 {
        self.get_slot_from_timestamp(self.get_current_timestamp())
    }

    /// Get the start timestamp of the current slot
    ///
    /// # Returns
    /// The start timestamp of the current slot in milliseconds
    ///
    /// # Example
    /// ```
    /// let current_slot_start = clock.get_current_slot_timestamp();
    /// ```
    fn get_current_slot_timestamp(&self) -> u64 {
        self.get_timestamp_from_slot(self.get_current_slot())
    }

    /// Get the start timestamp of the next slot
    ///
    /// # Returns
    /// The start timestamp of the next slot in milliseconds
    ///
    /// # Example
    /// ```
    /// let next_slot_start = clock.get_next_slot_timestamp();
    /// ```
    fn get_next_slot_timestamp(&self) -> u64 {
        self.get_timestamp_from_slot(self.get_current_slot() + 1)
    }

    /// Calculate the remaining time until the next slot starts
    ///
    /// # Returns
    /// The remaining time in milliseconds until the next slot starts
    ///
    /// # Example
    /// ```
    /// let remaining_time = clock.get_retain_time_to_next_slot();
    /// ```
    fn get_retain_time_to_next_slot(&self) -> u64 {
        self.get_next_slot_timestamp() - self.get_current_timestamp()
    }

    /// Calculate the elapsed time within the current slot
    ///
    /// # Returns
    /// The elapsed time in milliseconds since the current slot started
    ///
    /// # Example
    /// ```
    /// let elapsed_time = clock.get_current_slot_elapsed_time();
    /// ```
    fn get_current_slot_elapsed_time(&self) -> u64 {
        self.get_current_timestamp() - self.get_current_slot_timestamp()
    }

    /// Calculate the remaining time until a specific slot starts
    ///
    /// # Arguments
    /// * `slot` - The target slot number
    ///
    /// # Returns
    /// `Ok(remaining_time)` if the slot is in the future, `Err` if the slot is in the past
    ///
    /// # Example
    /// ```
    /// match clock.get_retain_time_to_slot(5) {
    ///     Ok(time) => println!("Time until slot 5: {}ms", time),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn get_retain_time_to_slot(&self, slot: u64) -> anyhow::Result<u64> {
            if slot < self.get_current_slot() {
            return Err(anyhow::anyhow!("slot is too old"));
        }
        Ok(self.get_timestamp_from_slot(slot) - self.get_current_timestamp())
    }

    /// Check if there is enough time to reach the next slot considering network cost
    ///
    /// # Returns
    /// `true` if the remaining time is less than or equal to NETWORK_COST_TIME_MS
    ///
    /// # Example
    /// ```
    /// if clock.is_can_reach_to_next_slot() {
    ///     println!("Can reach next slot");
    /// }
    /// ```
    fn is_can_reach_to_next_slot(&self) -> bool {
        self.get_retain_time_to_next_slot() >= NETWORK_COST_TIME_MS
    }

    /// Check if there is enough time to reach a specific slot considering network cost
    ///
    /// # Arguments
    /// * `slot` - The target slot number
    ///
    /// # Returns
    /// `Ok(true)` if there is enough time to reach the slot, `Ok(false)` if not enough time,
    /// `Err` if the slot is in the past
    ///
    /// # Example
    /// ```
    /// match clock.is_can_reach_to_slot(5) {
    ///     Ok(can_reach) => println!("Can reach slot 5: {}", can_reach),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn is_can_reach_to_slot(&self, slot: u64) -> anyhow::Result<bool> {
        Ok(self.get_retain_time_to_slot(slot)? >= NETWORK_COST_TIME_MS)
    }

    /// Get the start timestamp of the previous slot
    ///
    /// # Returns
    /// The start timestamp of the previous slot. If current slot is 0, returns the same as current slot
    ///
    /// # Example
    /// ```
    /// let prev_slot_start = clock.get_previous_slot_timestamp();
    /// ```
    fn get_previous_slot_timestamp(&self) -> u64 {
        self.get_timestamp_from_slot(self.get_current_slot().saturating_sub(1))
    }

    /// Get the end timestamp of a specific slot
    ///
    /// # Arguments
    /// * `slot` - The slot number
    ///
    /// # Returns
    /// The end timestamp of the specified slot (inclusive)
    ///
    /// # Example
    /// ```
    /// let slot_end = clock.get_slot_end_timestamp(5);
    /// ```
    fn get_slot_end_timestamp(&self, slot: u64) -> u64 {
        self.get_timestamp_from_slot(slot + 1) - 1
    }

    /// Check if a timestamp falls within a specific slot
    ///
    /// # Arguments
    /// * `timestamp` - The timestamp to check
    /// * `slot` - The slot number to check against
    ///
    /// # Returns
    /// `true` if the timestamp is within the specified slot, `false` otherwise
    ///
    /// # Example
    /// ```
    /// let is_in_slot = clock.is_in_slot(1753891200000, 0);
    /// ```
    fn is_in_slot(&self, timestamp: u64, slot: u64) -> bool {
        slot == self.get_slot_from_timestamp(timestamp)
    }

    /// Calculate the remaining time in the current slot
    ///
    /// # Returns
    /// The remaining time in milliseconds until the current slot ends
    ///
    /// # Example
    /// ```
    /// let remaining_in_slot = clock.get_current_slot_remaining_time();
    /// ```
    fn get_current_slot_remaining_time(&self) -> u64 {
        self.get_slot_end_timestamp(self.get_current_slot()) - self.get_current_timestamp()
    }

    /// Calculate the number of slots between two slot numbers
    ///
    /// # Arguments
    /// * `from_slot` - The starting slot number
    /// * `to_slot` - The ending slot number
    ///
    /// # Returns
    /// The number of slots between the two slot numbers (inclusive of to_slot, exclusive of from_slot)
    ///
    /// # Example
    /// ```
    /// let slot_count = clock.get_slots_between(1, 5); // Returns 4
    /// ```
    fn get_slots_between(&self, from_slot: u64, to_slot: u64) -> u64 {
        to_slot.saturating_sub(from_slot)
    }

    /// Get the duration of a slot in milliseconds
    ///
    /// # Returns
    /// The duration of a slot in milliseconds
    ///
    /// # Example
    /// ```rust
    /// let duration = clock.get_slot_duration(); // Returns 6000
    /// ```
    fn get_slot_duration(&self) -> u64 {
        SLOT_SIZE
    }

    /// Check if a timestamp is valid
    ///
    /// # Arguments
    /// * `timestamp` - The timestamp to validate
    ///
    /// # Returns
    /// `true` if the timestamp is valid, `false` otherwise
    ///
    /// # Example
    /// ```rust
    /// let is_valid = clock.is_valid_timestamp(1753891200000); // Returns true
    /// let is_valid = clock.is_valid_timestamp(0); // Returns false
    /// ```
    fn is_valid_timestamp(&self, timestamp: u64) -> bool {
        timestamp >= SLOT0_TIMESTAMP
    }

    /// Check if two timestamps are consecutive (in consecutive slots)
    ///
    /// # Arguments
    /// * `timestamp1` - The first timestamp
    /// * `timestamp2` - The second timestamp
    ///
    /// # Returns
    /// `true` if the timestamps are in consecutive slots, `false` otherwise
    ///
    /// # Example
    /// ```rust
    /// let are_consecutive = clock.are_timestamps_consecutive(1753891200000, 1753897200000); // Returns true
    /// let are_consecutive = clock.are_timestamps_consecutive(1753891200000, 1753903200000); // Returns false
    /// ```
    fn are_timestamps_consecutive(&self, timestamp1: u64, timestamp2: u64) -> bool {
        let slot1 = self.get_slot_from_timestamp(timestamp1);
        let slot2 = self.get_slot_from_timestamp(timestamp2);
        slot2 == slot1 + 1
    }

    /// Check if two timestamps are in the same slot
    ///
    /// # Arguments
    /// * `timestamp1` - The first timestamp
    /// * `timestamp2` - The second timestamp
    ///
    /// # Returns
    /// `true` if the timestamps are in the same slot, `false` otherwise
    ///
    /// # Example
    /// ```rust
    /// let same_slot = clock.are_timestamps_in_same_slot(1753891200000, 1753893000000); // Returns true
    /// let same_slot = clock.are_timestamps_in_same_slot(1753891200000, 1753897200000); // Returns false
    /// ```
    fn are_timestamps_in_same_slot(&self, timestamp1: u64, timestamp2: u64) -> bool {
        let slot1 = self.get_slot_from_timestamp(timestamp1);
        let slot2 = self.get_slot_from_timestamp(timestamp2);
        slot1 == slot2
    }

    /// Get the slot difference between two timestamps
    ///
    /// # Arguments
    /// * `timestamp1` - The first timestamp
    /// * `timestamp2` - The second timestamp
    ///
    /// # Returns
    /// The number of slots between the two timestamps
    ///
    /// # Example
    /// ```rust
    /// let slot_diff = clock.get_timestamp_slot_difference(1753891200000, 1753903200000); // Returns 2
    /// let slot_diff = clock.get_timestamp_slot_difference(1753903200000, 1753891200000); // Returns 2
    /// ```
    fn get_timestamp_slot_difference(&self, timestamp1: u64, timestamp2: u64) -> u64 {
        let slot1 = self.get_slot_from_timestamp(timestamp1);
        let slot2 = self.get_slot_from_timestamp(timestamp2);
        (slot2 as i64 - slot1 as i64).abs() as u64
    }

    /// Get the time difference between two slots
    ///
    /// # Arguments
    /// * `slot1` - The first slot number
    /// * `slot2` - The second slot number
    ///
    /// # Returns
    /// The time difference in milliseconds between the two slots
    ///
    /// # Example
    /// ```rust
    /// let time_diff = clock.get_slot_time_difference(0, 5); // Returns 30000 (5 * SLOT_SIZE)
    /// let time_diff = clock.get_slot_time_difference(5, 0); // Returns 30000 (5 * SLOT_SIZE)
    /// ```
    fn get_slot_time_difference(&self, slot1: u64, slot2: u64) -> u64 {
        (slot2 as i64 - slot1 as i64).abs() as u64 * SLOT_SIZE
    }
}

impl<T: Clock> Slot for T {}

#[derive(Debug, Clone, Copy)]
pub struct LocalClock;

impl Clock for LocalClock {
    fn get_current_timestamp(&self) -> u64 {
        Utc::now().timestamp_millis() as u64
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Instant(pub u64);

impl Instant {
    pub fn new(timestamp: u64) -> Self {
        Self(timestamp)
    }
}

impl Clock for Instant {
    fn get_current_timestamp(&self) -> u64 {
        self.0
    }
}


#[cfg(test)]
mod tests {
    use crate::common::slot::{Clock, Slot, SLOT0_TIMESTAMP, SLOT_SIZE, NETWORK_COST_TIME_MS};

    pub struct TestClock {
        timestamp: u64,
    }

    impl TestClock {
        fn new(timestamp: u64) -> Self {
            Self { timestamp }
        }
    }

    impl Clock for TestClock {
        fn get_current_timestamp(&self) -> u64 {
            self.timestamp
        }
    }

    #[test]
    fn test_get_slot_from_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_slot_from_timestamp(SLOT0_TIMESTAMP), 0);
        assert_eq!(clock.get_slot_from_timestamp(SLOT0_TIMESTAMP + SLOT_SIZE), 1);
        assert_eq!(clock.get_slot_from_timestamp(SLOT0_TIMESTAMP + SLOT_SIZE * 10), 10);
        assert_eq!(clock.get_slot_from_timestamp(SLOT0_TIMESTAMP + SLOT_SIZE / 2), 0);
    }

    #[test]
    fn test_get_timestamp_from_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_timestamp_from_slot(0), SLOT0_TIMESTAMP);
        assert_eq!(clock.get_timestamp_from_slot(1), SLOT0_TIMESTAMP + SLOT_SIZE);
        assert_eq!(clock.get_timestamp_from_slot(10), SLOT0_TIMESTAMP + SLOT_SIZE * 10);
    }

    #[test]
    fn test_get_current_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_current_slot(), 0);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE);
        assert_eq!(clock.get_current_slot(), 1);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE * 5 + 1000);
        assert_eq!(clock.get_current_slot(), 5);
    }

    #[test]
    fn test_get_current_slot_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_current_slot_timestamp(), SLOT0_TIMESTAMP);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE + 1000);
        assert_eq!(clock.get_current_slot_timestamp(), SLOT0_TIMESTAMP + SLOT_SIZE);
    }

    #[test]
    fn test_get_next_slot_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_next_slot_timestamp(), SLOT0_TIMESTAMP + SLOT_SIZE);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE);
        assert_eq!(clock.get_next_slot_timestamp(), SLOT0_TIMESTAMP + SLOT_SIZE * 2);
    }

    #[test]
    fn test_get_retain_time_to_next_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_retain_time_to_next_slot(), SLOT_SIZE);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE / 2);
        assert_eq!(clock.get_retain_time_to_next_slot(), SLOT_SIZE / 2);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE - 100);
        assert_eq!(clock.get_retain_time_to_next_slot(), 100);
    }

    #[test]
    fn test_get_current_slot_elapsed_time() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_current_slot_elapsed_time(), 0);

        let clock = TestClock::new(SLOT0_TIMESTAMP + 1000);
        assert_eq!(clock.get_current_slot_elapsed_time(), 1000);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE / 2);
        assert_eq!(clock.get_current_slot_elapsed_time(), SLOT_SIZE / 2);
    }

    #[test]
    fn test_get_retain_time_to_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test valid slot
        let result = clock.get_retain_time_to_slot(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SLOT_SIZE);

        // Test current slot
        let result = clock.get_retain_time_to_slot(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Test future slot
        let result = clock.get_retain_time_to_slot(5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SLOT_SIZE * 5);

        // Test past slot (should fail)
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE * 10);
        let result = clock.get_retain_time_to_slot(5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("slot is too old"));
    }

    #[test]
    fn test_is_can_reach_to_next_slot() {
        // Test when there's enough time (should return true)
        let clock = TestClock::new(SLOT0_TIMESTAMP + 100);
        assert_eq!(clock.is_can_reach_to_next_slot(), true);

        // Test when there's not enough time (should return false)
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE - NETWORK_COST_TIME_MS + 100);
        assert_eq!(clock.is_can_reach_to_next_slot(), false);

        // Test exactly at the threshold
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE - NETWORK_COST_TIME_MS);
        assert_eq!(clock.is_can_reach_to_next_slot(), true);
    }

    #[test]
    fn test_is_can_reach_to_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test valid future slot with enough time
        let result = clock.is_can_reach_to_slot(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Test valid future slot without enough time
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE - NETWORK_COST_TIME_MS + 100);
        let result = clock.is_can_reach_to_slot(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // Test invalid past slot
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE * 10);
        let result = clock.is_can_reach_to_slot(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_cases() {
        // Test very large slot numbers
        let large_slot = 1000000;
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        let timestamp = clock.get_timestamp_from_slot(large_slot);
        let slot = clock.get_slot_from_timestamp(timestamp);
        assert_eq!(slot, large_slot);

        // Test timestamp exactly at slot boundary
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE);
        assert_eq!(clock.get_current_slot(), 1);
        assert_eq!(clock.get_current_slot_elapsed_time(), 0);

        // Test timestamp in the middle of a slot
        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE / 2);
        assert_eq!(clock.get_current_slot(), 0);
        assert_eq!(clock.get_current_slot_elapsed_time(), SLOT_SIZE / 2);
    }

    #[test]
    fn test_local_clock() {
        use crate::common::slot::LocalClock;

        let clock = LocalClock;
        let timestamp = clock.get_current_timestamp();

        // Verify timestamp is reasonable (not too old, not too far in future)
        let now = chrono::Utc::now().timestamp_millis() as u64;
        assert!(timestamp >= now - 1000); // Within 1 second
        assert!(timestamp <= now + 1000); // Within 1 second
    }

    #[test]
    fn test_get_previous_slot_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        // When current timestamp is SLOT0_TIMESTAMP, current slot is 0, so previous slot is -1
        // But since we use saturating_sub(1), it becomes 0, so previous slot timestamp is SLOT0_TIMESTAMP
        assert_eq!(clock.get_previous_slot_timestamp(), SLOT0_TIMESTAMP);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE);
        assert_eq!(clock.get_previous_slot_timestamp(), SLOT0_TIMESTAMP);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE * 5);
        assert_eq!(clock.get_previous_slot_timestamp(), SLOT0_TIMESTAMP + SLOT_SIZE * 4);
    }

    #[test]
    fn test_get_slot_end_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        assert_eq!(clock.get_slot_end_timestamp(0), SLOT0_TIMESTAMP + SLOT_SIZE - 1);
        assert_eq!(clock.get_slot_end_timestamp(1), SLOT0_TIMESTAMP + SLOT_SIZE * 2 - 1);
        assert_eq!(clock.get_slot_end_timestamp(10), SLOT0_TIMESTAMP + SLOT_SIZE * 11 - 1);
    }

    #[test]
    fn test_is_in_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test timestamp in slot 0
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP, 0));
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE / 2, 0));
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE - 1, 0));

        // Test timestamp in slot 1
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE, 1));
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE + SLOT_SIZE / 2, 1));
        assert!(clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE * 2 - 1, 1));

        // Test timestamp not in slot
        assert!(!clock.is_in_slot(SLOT0_TIMESTAMP + SLOT_SIZE, 0));
        assert!(!clock.is_in_slot(SLOT0_TIMESTAMP, 1));
    }

    #[test]
    fn test_get_current_slot_remaining_time() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        // Current slot end timestamp is SLOT0_TIMESTAMP + SLOT_SIZE - 1
        // Remaining time = (SLOT0_TIMESTAMP + SLOT_SIZE - 1) - SLOT0_TIMESTAMP = SLOT_SIZE - 1
        assert_eq!(clock.get_current_slot_remaining_time(), SLOT_SIZE - 1);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE / 2);
        assert_eq!(clock.get_current_slot_remaining_time(), SLOT_SIZE / 2 - 1);

        let clock = TestClock::new(SLOT0_TIMESTAMP + SLOT_SIZE - 100);
        assert_eq!(clock.get_current_slot_remaining_time(), 99);
    }

    #[test]
    fn test_get_slots_between() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        assert_eq!(clock.get_slots_between(0, 5), 5);
        assert_eq!(clock.get_slots_between(5, 10), 5);
        assert_eq!(clock.get_slots_between(0, 0), 0);
        assert_eq!(clock.get_slots_between(10, 5), 0); // saturating_sub handles underflow
    }

    #[test]
    fn test_get_slot_duration() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert_eq!(clock.get_slot_duration(), SLOT_SIZE);
    }

    #[test]
    fn test_is_valid_timestamp() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);
        assert!(clock.is_valid_timestamp(SLOT0_TIMESTAMP));
        assert!(clock.is_valid_timestamp(SLOT0_TIMESTAMP + SLOT_SIZE));
        assert!(clock.is_valid_timestamp(SLOT0_TIMESTAMP + SLOT_SIZE * 10));
        assert!(!clock.is_valid_timestamp(SLOT0_TIMESTAMP - 1));
        assert!(!clock.is_valid_timestamp(0));
        // Test with very large future timestamp (should be valid now)
        assert!(clock.is_valid_timestamp(SLOT0_TIMESTAMP + 365 * 24 * 60 * 60 * 1000));
        assert!(clock.is_valid_timestamp(SLOT0_TIMESTAMP + 365 * 24 * 60 * 60 * 1000 + 1));
    }

    #[test]
    fn test_are_timestamps_consecutive() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test consecutive slots
        assert!(clock.are_timestamps_consecutive(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE));
        assert!(clock.are_timestamps_consecutive(SLOT0_TIMESTAMP + SLOT_SIZE, SLOT0_TIMESTAMP + SLOT_SIZE * 2));

        // Test non-consecutive slots
        assert!(!clock.are_timestamps_consecutive(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE * 2));
        assert!(!clock.are_timestamps_consecutive(SLOT0_TIMESTAMP + SLOT_SIZE, SLOT0_TIMESTAMP));

        // Test same slot
        assert!(!clock.are_timestamps_consecutive(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + 1000));
    }

    #[test]
    fn test_are_timestamps_in_same_slot() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test same slot
        assert!(clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + 1000));
        assert!(clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP + 1000, SLOT0_TIMESTAMP + 2000));

        // Test different slots
        assert!(!clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE));
        assert!(!clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP + SLOT_SIZE, SLOT0_TIMESTAMP));

        // Test edge cases
        assert!(clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE - 1));
        assert!(!clock.are_timestamps_in_same_slot(SLOT0_TIMESTAMP + SLOT_SIZE - 1, SLOT0_TIMESTAMP + SLOT_SIZE));
    }

    #[test]
    fn test_get_timestamp_slot_difference() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test slot differences
        assert_eq!(clock.get_timestamp_slot_difference(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE), 1);
        assert_eq!(clock.get_timestamp_slot_difference(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + SLOT_SIZE * 5), 5);
        assert_eq!(clock.get_timestamp_slot_difference(SLOT0_TIMESTAMP + SLOT_SIZE * 5, SLOT0_TIMESTAMP), 5);

        // Test same slot
        assert_eq!(clock.get_timestamp_slot_difference(SLOT0_TIMESTAMP, SLOT0_TIMESTAMP + 1000), 0);
        assert_eq!(clock.get_timestamp_slot_difference(SLOT0_TIMESTAMP + 1000, SLOT0_TIMESTAMP), 0);
    }

    #[test]
    fn test_get_slot_time_difference() {
        let clock = TestClock::new(SLOT0_TIMESTAMP);

        // Test time differences
        assert_eq!(clock.get_slot_time_difference(0, 1), SLOT_SIZE);
        assert_eq!(clock.get_slot_time_difference(1, 0), SLOT_SIZE);
        assert_eq!(clock.get_slot_time_difference(0, 5), SLOT_SIZE * 5);
        assert_eq!(clock.get_slot_time_difference(5, 0), SLOT_SIZE * 5);
        assert_eq!(clock.get_slot_time_difference(10, 15), SLOT_SIZE * 5);
        assert_eq!(clock.get_slot_time_difference(15, 10), SLOT_SIZE * 5);

        // Test same slot
        assert_eq!(clock.get_slot_time_difference(5, 5), 0);
    }
}
