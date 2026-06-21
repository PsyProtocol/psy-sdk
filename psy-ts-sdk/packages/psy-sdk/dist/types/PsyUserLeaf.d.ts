import type { QHashOut } from "./QHashOut";
export type PsyUserLeaf = {
    public_key: QHashOut;
    user_state_tree_root: QHashOut;
    balance: bigint;
    nonce: bigint;
    last_checkpoint_id: bigint;
    event_index: bigint;
    user_id: bigint;
};
//# sourceMappingURL=PsyUserLeaf.d.ts.map