use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct BlockHeightManager {
    current_height: Arc<AtomicU64>,
}

impl Default for BlockHeightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockHeightManager {
    pub fn new() -> Self {
        Self {
            current_height: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn update_height(&self, new_height: u64) -> bool {
        let current = self.current_height.fetch_max(new_height, Ordering::AcqRel);

        if new_height > current {
            trace!("Block height updated: {} -> {}", current, new_height);
            true
        } else {
            trace!("New height {} not greater than current {}", new_height, current);
            false
        }
    }

    pub fn get_height(&self) -> u64 {
        self.current_height.load(Ordering::Acquire)
    }

    pub fn set_height(&self, height: u64) {
        self.current_height.store(height, Ordering::Release);
        debug!("Block height set to {}", height);
    }
}
