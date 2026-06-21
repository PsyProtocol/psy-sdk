import { SCNumberLike } from "../core";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { PsyUserLeaf } from "../types";
interface IUserCacheRecord {
    localNonce: bigint;
    remoteNonce: bigint;
    localBalance: bigint;
    remoteBalance: bigint;
    unprocessedCheckpointId: bigint;
    unprocessedOutflows: bigint;
    unprocessedInflows: bigint;
}
declare class UserWalletCache {
    cache: Record<number, IUserCacheRecord>;
    getUserCached(userId: number): IUserCacheRecord;
    refreshUserFull(rpc: IRealmEdgeRpcProvider, userId: number): Promise<{
        cache: IUserCacheRecord;
        user: PsyUserLeaf;
    }>;
    refreshUser(rpc: IRealmEdgeRpcProvider, userId: number): Promise<IUserCacheRecord>;
    incNonce(userId: number): bigint;
    processTransfer(rpc: IRealmEdgeRpcProvider, sender: SCNumberLike, recipient: SCNumberLike, amount: SCNumberLike): Promise<bigint>;
}
declare const userWalletCache: UserWalletCache;
export { userWalletCache };
export type { IUserCacheRecord };
//# sourceMappingURL=cache.d.ts.map