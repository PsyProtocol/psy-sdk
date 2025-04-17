use qed_core::job::id::ProvingJobCircuitType;
use qed_core::{
    config::network_constants::{
        DEFAULT_USER_STATE_TREE_ROOT, REALM_API_GUTA_FROM_USER_CHANNEL_ID,
        REALM_API_UPDATE_CONTRACT_STATE_TREE_CHANNEL_ID, REALM_USER_TREE_HEIGHT,
    },
    data::qhashout::QHashOut,
};
use qed_crypto::common::cached_circuit_library::get_cached_circuit_library;
use qed_crypto::common::circuit_library::CircuitInfoLibraryCore;
use qed_store::config::store_config::QEDFelt;
use serde::{Deserialize, Serialize};

type F = QEDFelt;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RealmConfig {
    pub rpc_node_id: u32,
    pub realm_id: u32,

    pub users_per_realm: usize,
    pub realm_root_level: u8,
    pub guta_channel_id: u64,
    pub guta_circuit_whitelist: QHashOut<F>,
    pub default_user_state_tree_root: QHashOut<F>,
    pub contract_state_tree_update_channel_id: u64,
}

impl RealmConfig {
    pub fn get_standard(rpc_node_id: u32, realm_id: u32) -> Self {
        let library = get_cached_circuit_library::<F>();

        let realm_root_level = REALM_USER_TREE_HEIGHT;
        let users_per_realm = 1usize << (REALM_USER_TREE_HEIGHT as usize);

        Self {
            rpc_node_id,
            users_per_realm,
            realm_root_level,
            guta_channel_id: REALM_API_GUTA_FROM_USER_CHANNEL_ID,
            guta_circuit_whitelist: library
                .get_group_inclusion_proof(
                    ProvingJobCircuitType::GUTATwoGUTA,
                    ProvingJobCircuitType::GUTATwoGUTA,
                )
                .unwrap()
                .root,
            realm_id,
            default_user_state_tree_root: DEFAULT_USER_STATE_TREE_ROOT,
            contract_state_tree_update_channel_id: REALM_API_UPDATE_CONTRACT_STATE_TREE_CHANNEL_ID,
        }
    }

    pub fn includes_user_id(&self, id: u64) -> bool {
        let r64 = self.realm_id as u64;
        id >= r64 * (self.users_per_realm as u64) && id < (r64 + 1) * (self.users_per_realm as u64)
    }

    pub fn get_local_user_id_masked(&self, global_user_id: u64) -> u64 {
        global_user_id & ((1u64 << (self.realm_root_level as u64)) - 1u64)
    }
}
