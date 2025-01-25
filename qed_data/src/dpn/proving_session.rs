use plonky2::hash::hash_types::RichField;
use qed_core::data::qhashout::QHashOut;
use serde::{Deserialize, Serialize};

use crate::qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf}, user::QEDUserLeaf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Default)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct DPNProvingSessionCheckpointState<F: RichField> {
    pub checkpoint_tree_root: QHashOut<F>,
    pub checkpoint_hash: QHashOut<F>,
    pub checkpoint_id: F,
    pub checkpoint_leaf: QEDCheckpointLeaf<F>,
    pub last_global_tree_state_roots: QEDCheckpointGlobalStateRoots<F>,
    pub session_start_user_leaf: QEDUserLeaf<F>,
}
