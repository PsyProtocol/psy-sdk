/*pub trait PsyTreeConfig {
    const CHECKPOINT_TREE_HEIGHT: u8;
    const GLOBAL_USER_TREE_HEIGHT: u8;
    const GLOBAL_CONTRACT_TREE_HEIGHT: u8;
    const GLOBAL_DEPOSIT_TREE_HEIGHT: u8;
    const GLOBAL_WITHDRAWAL_TREE_HEIGHT: u8;

    const CONTRACT_FUNCTION_TREE_HEIGHT: u8;

    const MAX_CONTRACT_STATE_TREE_HEIGHT: u8;


}

pub struct PsyTestnetTreeConfig;
impl PsyTreeConfig for PsyTestnetTreeConfig {
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

// Include the generated constants from build.rs
include!(concat!(env!("OUT_DIR"), "/generated_constants.rs"));

// Network Magic
pub const PSY_NETWORK_MAGIC_MAINNET: u64 = 0x1337CF514544C069u64;
pub const PSY_NETWORK_MAGIC_TESTNET: u64 = 0x1337CF514544C169u64;
pub const PSY_NETWORK_MAGIC_REGTEST: u64 = 0x1337CF514544CF69u64;

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
pub const DEFAULT_CALLER_CONTRACT_ID_U64: u64 = (u32::MAX as u64) + 1;

// used for signing a transaction/contract function call 0xCA11_<ascii'TXCALL'>
pub const SIGN_SIMPLE_TRANSACTION_MAGIC: u64 = 0xCA11_545843414C4C;

pub const UPS_SESSION_PROOF_TREE_HEIGHT: u8 = 16;

pub const UPS_CIRCUIT_WHITELIST_TREE_HEIGHT: u8 = 8;

pub const PSY_SIG_ACTION_SIGN_UPS_END_CAP: u64 = 0x51454445434150CFu64;

pub const VM_TYPE_STANRDARD_DAPEN_V1: u32 = 1;

pub const GUTA_CIRCUIT_WHITELIST_TREE_HEIGHT: u8 = 4;

pub const DEFAULT_USER_STATE_TREE_ROOT_U64: [u64; 4] = [3896366420105793420, 17410332186442776169, 7329967984378645716, 6310665049578686403];
pub const DEFAULT_USER_STATE_TREE_ROOT: QHashOut<GoldilocksField> = QHashOut::<GoldilocksField>(HashOut {
    elements: [
        GoldilocksField(3896366420105793420),
        GoldilocksField(17410332186442776169),
        GoldilocksField(7329967984378645716),
        GoldilocksField(6310665049578686403),
    ],
});

pub const TOKEN_CONTRACT_ID: u32 = 0;
pub const TOKEN_SIMPLE_BURN_METHOD_ID: u32 = 2923993647;

// start circuit specific config
pub const BATCH_USER_REGISTRAITION_SUB_TREE_HEIGHT: usize = 8;
pub const BATCH_USER_REGISTRAITION_MAX_SUB_TREES: usize = 4;

pub const BATCH_DEPLOY_CONTRACT_SUB_TREE_HEIGHT: usize = 8;

// start constant channels
pub const COORD_API_REGISTER_USER_CHANNEL_ID: u64 = 0xCC524547555352;
pub const COORD_API_DEPLOY_CONTRACT_CHANNEL_ID: u64 = 0xCC444550434F4E;
pub const COORD_API_GUTA_FROM_REALMS_CHANNEL_ID: u64 = 0xCC475554414652;

pub const REALM_API_GUTA_FROM_USER_CHANNEL_ID: u64 = 0x22475554414652;
pub const REALM_API_UPDATE_CONTRACT_STATE_TREE_CHANNEL_ID: u64 = 0x22435354555044;

pub const PSY_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL: u64 = 0x901337123;
pub const CST_USER_UPDATE_CHANNEL_ID: u64 = 0x101337;

pub const COORDINATOR_EDGE_TO_PROCESSOR_CHANNEL: u64 = 0xCC544F5245414C4D;

pub const REALM_PROCESSOR_TO_EDGE_CHANNEL: u64 = 0x524C4D50524F4F46;

// move cil parameter
pub const MAX_PROCESSED_END_CAPS_PER_BLOCK: u64 = 16;
pub const MAX_PROCESSED_CONTRACTS_PER_BLOCK: u64 = 64;
pub const MAX_PROCESSED_USERS_PER_BLOCK: u64 = 256;

// slot constants
pub const SLOT0: u64 = 0;
pub const SLOT_SIZE: u64 = 2000; // 2s
pub const REALM_SLOT_SIZE_MS: u64 = 3000; // 3s
pub const SLOT0_TIMESTAMP: u64 = 1753891200000; // 2025-07-31 00:00:00
pub const NETWORK_COST_TIME_MS: u64 = 1000; // 1s

use plonky2::{field::types::Field, hash::hash_types::RichField};

pub fn get_default_worker_public_key<F: RichField>() -> QHashOut<F> {
    QHashOut::from_values(1, 1, 1, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_constants() {
        // Verify constants are generated correctly from config.json
        assert_eq!(NATIVE_CURRENCY_DECIMAL, 9);
        assert_eq!(NATIVE_CURRENCY, "0");
        assert_eq!(NATIVE_CURRENCY_NAME, "PSY");
        assert_eq!(REGISTER_USER_FEE, 0);
        assert_eq!(DEPLOY_CONTRACT_FEE, 0);
        assert_eq!(GUTA_FEE, 5000000000);
    }
}
