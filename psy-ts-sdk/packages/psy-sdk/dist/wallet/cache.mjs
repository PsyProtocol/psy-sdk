import { psyFelt } from '../utils/felt.mjs';
import '../utils/json.mjs';
import '../utils/random.mjs';

class UserWalletCache {
    constructor() {
        this.cache = {};
    }
    getUserCached(userId) {
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
    async refreshUserFull(rpc, userId) {
        const currentBlock = await rpc.getLatestBlockState();
        const user = await rpc.getUserLeafData(currentBlock.checkpoint_id, userId);
        const cachedUser = this.getUserCached(userId);
        if (currentBlock.checkpoint_id > cachedUser.unprocessedCheckpointId) {
            cachedUser.unprocessedCheckpointId = BigInt(currentBlock.checkpoint_id);
            cachedUser.unprocessedOutflows = BigInt(0);
            cachedUser.unprocessedInflows = BigInt(0);
            cachedUser.localBalance = psyFelt(user.balance);
        }
        cachedUser.remoteBalance = psyFelt(user.balance);
        cachedUser.remoteNonce = psyFelt(user.nonce);
        if (cachedUser.localNonce < cachedUser.remoteNonce) {
            cachedUser.localNonce = cachedUser.remoteNonce;
        }
        return { cache: cachedUser, user };
    }
    async refreshUser(rpc, userId) {
        return (await this.refreshUserFull(rpc, userId)).cache;
    }
    incNonce(userId) {
        const user = this.getUserCached(userId);
        user.localNonce += BigInt(1);
        return user.localNonce;
    }
    async processTransfer(rpc, sender, recipient, amount) {
        const senderUser = await this.refreshUser(rpc, Number(sender + ""));
        const recipientUser = await this.refreshUser(rpc, Number(recipient + ""));
        senderUser.localBalance -= psyFelt(amount);
        recipientUser.localBalance += psyFelt(amount);
        return this.incNonce(Number(sender + ""));
    }
}
const userWalletCache = new UserWalletCache();

export { userWalletCache };
