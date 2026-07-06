import { MultiCoordinatorRpcProvider } from "../coord-edge-rpc";
import { Felt } from "../core";
import { RpcConfig } from "../provider";
import { MultiRealmRpcProvider } from "../realm-edge-rpc";
export declare class RpcProvider {
    coordinatorRpcProvider: MultiCoordinatorRpcProvider;
    realmRpcProvider: MultiRealmRpcProvider;
    constructor(coordinatorConfig: RpcConfig[], realmConfig: RpcConfig[], userPerRealm: number);
    setUserId(userId: number): void;
    getLastClaimCheckpointId(checkpointId: Felt, userId: Felt): Promise<Felt>;
    getPsyBalance(checkpointId: Felt, userId: Felt): Promise<Felt>;
    checkTxIsConfirmed(checkpointId: Felt, pkHash: string, txHash: string): Promise<boolean>;
    getClaimAmount(checkpointId: Felt, userId: Felt, claimUserId: Felt): Promise<Felt>;
}
//# sourceMappingURL=provider.d.ts.map