/*pub trait QEDTreeConfig {
    const CHECKPOINT_TREE_HEIGHT: u8;
    const GLOBAL_USER_TREE_HEIGHT: u8;
    const GLOBAL_CONTRACT_TREE_HEIGHT: u8;
    const GLOBAL_DEPOSIT_TREE_HEIGHT: u8;
    const GLOBAL_WITHDRAWAL_TREE_HEIGHT: u8;

    const CONTRACT_FUNCTION_TREE_HEIGHT: u8;

    const MAX_CONTRACT_STATE_TREE_HEIGHT: u8;


}

pub struct QEDTestnetTreeConfig;
impl QEDTreeConfig for QEDTestnetTreeConfig {
    const CHECKPOINT_TREE_HEIGHT: u8 = 32;
    const GLOBAL_USER_TREE_HEIGHT: u8 = 24;
    const GLOBAL_CONTRACT_TREE_HEIGHT: u8 = 24;
    const GLOBAL_DEPOSIT_TREE_HEIGHT: u8 = 32;
    const GLOBAL_WITHDRAWAL_TREE_HEIGHT: u8 = 32;
    
    const CONTRACT_FUNCTION_TREE_HEIGHT: u8 = 16;

    const MAX_CONTRACT_STATE_TREE_HEIGHT: u8 = 32;
}
*/

pub const CHECKPOINT_TREE_HEIGHT: u8 = 32;
pub const GLOBAL_USER_TREE_HEIGHT: u8 = 24;
pub const GLOBAL_CONTRACT_TREE_HEIGHT: u8 = 24;
pub const GLOBAL_DEPOSIT_TREE_HEIGHT: u8 = 32;
pub const GLOBAL_WITHDRAWAL_TREE_HEIGHT: u8 = 32;

pub const CONTRACT_FUNCTION_TREE_HEIGHT: u8 = 16;

pub const MAX_CONTRACT_STATE_TREE_HEIGHT: u8 = 32;


// Number of blocks that a data availability mining challenge is open for
pub const DA_CHALLENGE_WINDOW: usize = 14;


pub const DEFERRED_CALL_MAGIC: u64 = 0xDEFE1212EDCA11u64;
pub const DEFERRED_TRANSACTION_TREE_HEIGHT: u8 = 16;
