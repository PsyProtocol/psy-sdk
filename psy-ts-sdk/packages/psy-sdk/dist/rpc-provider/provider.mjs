import '../coord-edge-rpc/types.mjs';
import { MultiCoordinatorRpcProvider } from '../coord-edge-rpc/client.mjs';
import '../realm-edge-rpc/types.mjs';
import { MultiRealmRpcProvider } from '../realm-edge-rpc/client.mjs';

class RpcProvider {
    constructor(coordinatorConfig, realmConfig, userPerRealm) {
        this.coordinatorRpcProvider = new MultiCoordinatorRpcProvider(coordinatorConfig);
        this.realmRpcProvider = new MultiRealmRpcProvider(realmConfig, userPerRealm);
    }
    setUserId(userId) {
        this.realmRpcProvider.setUserId(userId);
    }
    async getLastClaimCheckpointId(checkpointId, userId) {
        console.log("getLastClaimCheckpointId, checkpointId: " + checkpointId);
        console.log("getLastClaimCheckpointId, userId: " + userId);
        const leafHash = await this.realmRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeLeafHash(checkpointId, userId, 0, 32, 0);
        if (!leafHash || leafHash.length != 64) {
            console.warn("getLastClaimCheckpointId failed, leafHash.length != 64, leafHash: " + leafHash);
            throw new Error("getLastClaimCheckpointId failed, leafHash.length != 64");
        }
        console.log("getLastClaimCheckpointId, leafHash: " + leafHash);
        return parseInt(leafHash?.substring(32, 48), 16);
    }
    async getPsyBalance(checkpointId, userId) {
        const leafHash = await this.realmRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeLeafHash(checkpointId, userId, 0, 32, 0);
        if (!leafHash || leafHash.length != 64) {
            console.warn("getPsyBalance failed, leafHash.length != 64, leafHash: " + leafHash);
            throw new Error("getPsyBalance failed, leafHash.length != 64");
        }
        console.log("getPsyBalance, leafHash: " + leafHash);
        return parseInt(leafHash?.substring(48, 64), 16);
    }
    async checkTxIsConfirmed(checkpointId, pkHash, txHash) {
        const userId = await this.coordinatorRpcProvider.getUserId(pkHash);
        if (!userId) {
            console.warn("checkTxIsConfirmed failed, userId is undefined, pkHash: " + pkHash);
            throw new Error("checkTxIsConfirmed failed, userId is undefined");
        }
        const userLeafHash = await this.realmRpcProvider.getRpcProviderByUserId(userId).getUserTreeLeafHash(checkpointId, userId);
        if (!userLeafHash || userLeafHash.length != 64) {
            console.warn("checkTxIsConfirmed failed, userLeafHash.length != 64, userLeafHash: " + userLeafHash);
            throw new Error("checkTxIsConfirmed failed, userLeafHash.length != 64");
        }
        return userLeafHash == txHash;
    }
    async getClaimAmount(checkpointId, userId, claimUserId) {
        const contractId = 0;
        const height = 32;
        const senderTotalSentIndex = 3n + BigInt(userId) * 2n;
        const senderTotalSentSlot = senderTotalSentIndex / 4n;
        const senderTotalSentSlotIndex = 3n - senderTotalSentIndex % 4n;
        const amountClaimedIndex = 3n + BigInt(claimUserId) * 2n + 1n;
        const amountClaimedSlot = amountClaimedIndex / 4n;
        const amountClaimedSlotIndex = 3n - amountClaimedIndex % 4n;
        const userTotalSentSlotValue = await this.realmRpcProvider.getRpcProviderByUserId(claimUserId).getUserContractStateTreeLeafHash(checkpointId, claimUserId, contractId, height, senderTotalSentSlot);
        const userTotalSent = parseInt(userTotalSentSlotValue?.substring(Number(senderTotalSentSlotIndex) * 16, Number(senderTotalSentSlotIndex) * 16 + 16), 16);
        const amountClaimedSlotValue = await this.realmRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeLeafHash(checkpointId, userId, contractId, height, amountClaimedSlot);
        const amountClaimed = parseInt(amountClaimedSlotValue?.substring(Number(amountClaimedSlotIndex) * 16, Number(amountClaimedSlotIndex) * 16 + 16), 16);
        if (amountClaimed > userTotalSent) {
            throw new Error(`amount claimed ${amountClaimed} is greater than user total sent ${userTotalSent}`);
        }
        return userTotalSent - amountClaimed;
    }
}

export { RpcProvider };
