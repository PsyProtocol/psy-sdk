import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { SCNumberLike } from "../core";
import { QEDUserLeaf } from "../types";
import { qedFelt } from "../utils";

interface IUserCacheRecord {
    localNonce: bigint;
    remoteNonce: bigint;
    localBalance: bigint;
    remoteBalance: bigint;
    unprocessedCheckpointId: bigint;
    unprocessedOutflows: bigint;
    unprocessedInflows: bigint;
}

class UserWalletCache {
    cache: Record<number, IUserCacheRecord> = {};

    getUserCached(userId: number): IUserCacheRecord {
        if (!Object.hasOwnProperty.call(this.cache, userId)) {
            this.cache[userId] = {
                localNonce: BigInt(0),
                remoteNonce: BigInt(0),
                localBalance: BigInt(0),
                remoteBalance: BigInt(0),
                unprocessedCheckpointId: -1n,
                unprocessedOutflows: BigInt(0),
                unprocessedInflows: BigInt(0),
            };
        }
        return this.cache[userId];
    }

    async refreshUserFull(
        rpc: ICoordinatorEdgeRpcProvider,
        userId: number
    ): Promise<{ cache: IUserCacheRecord; user: QEDUserLeaf }> {
        const currentBlock = await rpc.getLatestCheckpoint();
        const user = await rpc.getUserLeafData(currentBlock.checkpoint_id, userId);
        const cachedUser = this.getUserCached(userId);
        if (currentBlock.checkpoint_id > cachedUser.unprocessedCheckpointId) {
            cachedUser.unprocessedCheckpointId = BigInt(currentBlock.checkpoint_id);
            cachedUser.unprocessedOutflows = BigInt(0);
            cachedUser.unprocessedInflows = BigInt(0);
            cachedUser.localBalance = qedFelt(user.balance);
        }
        cachedUser.remoteBalance = qedFelt(user.balance);
        cachedUser.remoteNonce = qedFelt(user.nonce);

        if (cachedUser.localNonce < cachedUser.remoteNonce) {
            cachedUser.localNonce = cachedUser.remoteNonce;
        }

        return { cache: cachedUser, user };
    }
    async refreshUser(rpc: ICoordinatorEdgeRpcProvider, userId: number): Promise<IUserCacheRecord> {
        return (await this.refreshUserFull(rpc, userId)).cache;
    }

    incNonce(userId: number): bigint {
        const user = this.getUserCached(userId);
        user.localNonce += BigInt(1);
        return user.localNonce;
    }

    async processTransfer(
        rpc: ICoordinatorEdgeRpcProvider,
        sender: SCNumberLike,
        recipient: SCNumberLike,
        amount: SCNumberLike
    ): Promise<bigint> {
        const senderUser = await this.refreshUser(rpc, Number(sender + ""));
        const recipientUser = await this.refreshUser(rpc, Number(recipient + ""));
        senderUser.localBalance -= qedFelt(amount);
        recipientUser.localBalance += qedFelt(amount);
        return this.incNonce(Number(sender + ""));
    }
}

const userWalletCache = new UserWalletCache();

export { userWalletCache };

export type { IUserCacheRecord };
