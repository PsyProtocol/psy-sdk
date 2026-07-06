import type { GUTAStats } from "./GUTAStats";
import type { PsyUserLeaf } from "./PsyUserLeaf";
import type { UPSEndCapResultCompact } from "./UPSEndCapResultCompact";
export type SubmitUserEndCapNonProofCoreInput = {
    checkpoint_id: bigint;
    stats: GUTAStats;
    state_transition: UPSEndCapResultCompact;
    new_user_leaf: PsyUserLeaf;
};
//# sourceMappingURL=SubmitUserEndCapNonProofCoreInput.d.ts.map