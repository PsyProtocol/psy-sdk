pub mod session;
pub use session::*;
pub mod utils;
use std::time::Duration;

pub use utils::*;

pub async fn sleep(dur: Duration) {
    tokio::time::sleep(dur).await;
}
