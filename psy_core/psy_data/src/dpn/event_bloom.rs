use std::io::Cursor;

use fastbloom_rs::{BloomFilter, FilterBuilder, Membership};
use plonky2::hash::hash_types::RichField;
use serde::{Deserialize, Serialize};
use zstd::{decode_all, encode_all};

use crate::dpn::event::PsyUserEventRecord;

pub const EVENT_KEYS_NUM: usize = 6;
pub const EXPECTED_EVENTS_CAPACITY: usize = 1 << 8;
pub const EXPECTED_FALSE_POSITIVE_RATE: f64 = 0.001;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct BloomConfig {
    pub false_positive_rate: f64,
    pub min_capacity: u64,
}

impl BloomConfig {
    pub fn new(capacity: usize, false_positive_rate: f64) -> Self {
        Self {
            false_positive_rate,
            min_capacity: capacity as u64,
        }
    }

    pub fn new_with_events_capacity(events_capacity: usize, false_positive_rate: f64) -> Self {
        Self::new(events_capacity * EVENT_KEYS_NUM, false_positive_rate)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventBloomFilter {
    inner: BloomFilter,
    config: BloomConfig,
}

impl EventBloomFilter {
    pub fn new(config: BloomConfig) -> Self {
        let inner = FilterBuilder::new(config.min_capacity, config.false_positive_rate).build_bloom_filter();

        Self { inner, config }
    }
    pub fn add_events<I: IntoIterator<Item = impl EventBloomItem>>(&mut self, events: I) -> anyhow::Result<()> {
        for event in events.into_iter() {
            for key in event.get_all_keys() {
                self.inner.add(&key);
            }
        }

        Ok(())
    }

    pub fn contains(&self, key: Vec<u8>) -> bool {
        self.inner.contains(&key)
    }

    pub fn config(&self) -> BloomConfig {
        self.config
    }

    pub fn inner(&self) -> &BloomFilter {
        &self.inner
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }

    pub fn to_compressed_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let serialized = self.to_bytes()?;
        let cursor = Cursor::new(&serialized);
        let compressed = encode_all(cursor, 3)?;
        Ok(compressed)
    }

    pub fn from_compressed_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let decompressed = decode_all(bytes)?;
        Self::from_bytes(&decompressed)
    }
}

pub trait EventBloomItem {
    fn user_id(&self) -> u64;
    fn contract_id(&self) -> u64;
    fn action(&self) -> u64;

    fn action_key(&self) -> Vec<u8> {
        self.action().to_le_bytes().to_vec()
    }

    fn contract_key(&self) -> Vec<u8> {
        self.contract_id().to_le_bytes().to_vec()
    }

    fn contract_action_key(&self) -> Vec<u8> {
        self.contract_id().to_le_bytes().into_iter().chain(self.action().to_le_bytes()).collect()
    }

    fn user_key(&self) -> Vec<u8> {
        self.user_id().to_le_bytes().to_vec()
    }

    fn user_contract_key(&self) -> Vec<u8> {
        self.user_id().to_le_bytes().into_iter().chain(self.contract_id().to_le_bytes()).collect()
    }

    fn user_contract_action_key(&self) -> Vec<u8> {
        self.user_id()
            .to_le_bytes()
            .into_iter()
            .chain(self.contract_id().to_le_bytes())
            .chain(self.action().to_le_bytes())
            .collect()
    }

    fn get_all_keys(&self) -> Vec<Vec<u8>> {
        vec![
            self.action_key(),
            self.contract_key(),
            self.contract_action_key(),
            self.user_key(),
            self.user_contract_key(),
            self.user_contract_action_key(),
        ]
    }
}

impl<T: EventBloomItem> EventBloomItem for &T {
    fn user_id(&self) -> u64 {
        (**self).user_id()
    }
    fn contract_id(&self) -> u64 {
        (**self).contract_id()
    }
    fn action(&self) -> u64 {
        (**self).action()
    }
}

impl<F: RichField> EventBloomItem for PsyUserEventRecord<F> {
    fn user_id(&self) -> u64 {
        self.user_id.to_canonical_u64()
    }

    fn contract_id(&self) -> u64 {
        self.contract_id.to_canonical_u64()
    }

    fn action(&self) -> u64 {
        self.method_id.to_canonical_u64()
    }
}

mod tests {
    use std::io::Cursor;

    use plonky2::{
        field::{goldilocks_field::GoldilocksField, types::PrimeField64},
        hash::hash_types::RichField,
    };
    use psy_common::traits::to_qfelts::ToQFelts;

    const EVENT_LEN: usize = 10;
    const USER_COUNT: usize = 1 << 16;
    const FALSE_POSITIVE_TEST_NUM: usize = 1 << 12;

    use crate::dpn::{
        event::PsyUserEventRecord,
        event_bloom::{BloomConfig, EventBloomFilter, EventBloomItem, EXPECTED_EVENTS_CAPACITY, EXPECTED_FALSE_POSITIVE_RATE},
    };

    fn random_event<F: RichField>() -> PsyUserEventRecord<F> {
        PsyUserEventRecord::from_qfelts(&F::rand_vec(EVENT_LEN))
    }

    #[test]
    fn test_event_bloom_filter() -> anyhow::Result<()> {
        let events: Vec<_> = (0..EXPECTED_EVENTS_CAPACITY).map(|_| random_event::<GoldilocksField>()).collect();

        let bloom_config = BloomConfig::new_with_events_capacity(EXPECTED_EVENTS_CAPACITY, EXPECTED_FALSE_POSITIVE_RATE);
        let mut bloom_filter: EventBloomFilter = EventBloomFilter::new(bloom_config);
        for _ in 0..USER_COUNT {
            bloom_filter.add_events(events.clone())?;

            for e in events.iter() {
                for key in e.get_all_keys().iter() {
                    assert!(bloom_filter.contains(key.clone()), "BloomFilter false negative on inserted event!");
                }
            }
        }

        // 2. Check for events that have not been inserted there may be false positives
        //    but the probability is extremely low
        let mut false_positive_count = 0;
        for _ in 0..FALSE_POSITIVE_TEST_NUM {
            let e = random_event::<GoldilocksField>();
            if !events.contains(&e) {
                for key in e.get_all_keys().iter() {
                    if bloom_filter.contains(key.clone()) {
                        false_positive_count += 1;
                    }
                }
            }
        }

        let false_positive_rate = false_positive_count as f64 / FALSE_POSITIVE_TEST_NUM as f64;
        println!(
            "total events: {}, false positive count: {}, false positive rate: {}",
            FALSE_POSITIVE_TEST_NUM, false_positive_count, false_positive_rate
        );

        let bloom_bytes = bloom_filter.to_bytes()?;
        let compressed = bloom_filter.to_compressed_bytes()?;

        println!(
            "BloomFilters original {} bytes, compressed {} bytes, compressed ratio: {}",
            bloom_bytes.len(),
            compressed.len(),
            1.0 - compressed.len() as f64 / bloom_bytes.len() as f64
        );

        let bloom_filter_decompressed: EventBloomFilter = EventBloomFilter::from_compressed_bytes(&compressed)?;

        assert_eq!(bincode::serialize(&bloom_filter)?, bincode::serialize(&bloom_filter_decompressed)?);
        Ok(())
    }
}
