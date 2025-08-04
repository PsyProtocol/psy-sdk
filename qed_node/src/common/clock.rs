use std::ops::Deref;
use tokio::time::{sleep, Duration};
use crate::common::slot::{Clock, LocalClock, Slot};

pub struct SlotTimer<T> {
    clock: T,
}

impl<T: Clock> SlotTimer<T> {
    pub fn new(clock: T) -> Self {
        Self {
            clock,
        }
    }
    
   pub async fn wait_for_next_slot(&self) -> u64 {
        let remain_time = self.clock.get_retain_time_to_next_slot();
        sleep(Duration::from_millis(remain_time)).await;// todo optimize sleep
        self.clock.get_current_slot()
    }
    
    pub async fn run_slot_timer<F,Fut>(&self, callback: F)
    where
        F: Fn(u64) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        loop {
            let current_slot =  self.wait_for_next_slot().await;
            callback(current_slot).await;
        }
    }
}

impl<T: Clock> Deref for SlotTimer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.clock
    }
}



