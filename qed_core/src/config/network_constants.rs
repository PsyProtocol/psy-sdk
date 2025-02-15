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

use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::HashOut};

use crate::data::qhashout::QHashOut;


// Network Magic
pub const QED_NETWORK_MAGIC_MAINNET: u64 = 0x1337CF514544C069u64;
pub const QED_NETWORK_MAGIC_TESTNET: u64 = 0x1337CF514544C169u64;
pub const QED_NETWORK_MAGIC_REGTEST: u64 = 0x1337CF514544CF69u64;



pub const GLOBAL_USER_TREE_HEIGHT: u8 = 24;
pub const COORDINATOR_USER_TREE_HEIGHT: u8 = 12;
pub const REALM_USER_TREE_HEIGHT: u8 = 12;


pub const CHECKPOINT_TREE_HEIGHT: u8 = 32;
pub const GLOBAL_CONTRACT_TREE_HEIGHT: u8 = 24;
pub const GLOBAL_DEPOSIT_TREE_HEIGHT: u8 = 32;
pub const GLOBAL_WITHDRAWAL_TREE_HEIGHT: u8 = 32;

pub const CONTRACT_FUNCTION_TREE_HEIGHT: u8 = 16;

pub const MAX_CONTRACT_STATE_TREE_HEIGHT: u8 = 32;


// Number of blocks that a data availability mining challenge is open for
pub const DA_CHALLENGE_WINDOW: usize = 14;


pub const DEFERRED_CALL_MAGIC: u64 = 0xDEFE1212EDCA11u64;
pub const DEFERRED_TRANSACTION_TREE_HEIGHT: u8 = 16;
pub const INLINE_TRANSACTION_TREE_HEIGHT: u8 = 16;


// used for signing a transaction/contract function call 0xCA11_<ascii'TXCALL'>
pub const SIGN_SIMPLE_TRANSACTION_MAGIC: u64 = 0xCA11_545843414C4C;


pub const UPS_SESSION_PROOF_TREE_HEIGHT: u8 = 16;

pub const UPS_CIRCUIT_WHITELIST_TREE_HEIGHT: u8 = 8;


pub const QED_SIG_ACTION_SIGN_UPS_END_CAP: u64 = 0x51454445434150CFu64;


pub const VM_TYPE_STANRDARD_DAPEN_V1: u32 = 1;


pub const GUTA_CIRCUIT_WHITELIST_TREE_HEIGHT: u8 = 4;



pub const DEFAULT_USER_STATE_TREE_ROOT_U64: [u64; 4] = [
    3896366420105793420,
    17410332186442776169,
    7329967984378645716,
    6310665049578686403,
];
pub const DEFAULT_USER_STATE_TREE_ROOT: QHashOut<GoldilocksField> = QHashOut::<GoldilocksField>(
    HashOut {
        elements: [
            GoldilocksField(3896366420105793420),
            GoldilocksField(17410332186442776169),
            GoldilocksField(7329967984378645716),
            GoldilocksField(6310665049578686403),
        ],
    }
);

