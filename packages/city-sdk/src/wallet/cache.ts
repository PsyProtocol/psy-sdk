import { ICityRPCProvider } from "../rpc/types";
import { DEPOSIT_FEE_AMOUNT, MAX_CHECKPOINT_ID, WITHDRAWAL_FEE_AMOUNT } from "../constants";
import { cityFelt } from "../utils/felt";
import { ICityUserState, SCNumberLike } from "../rpc/baseTypes";
import { ICityCompleteUserInfo } from "./types";

interface IUserCacheRecord {
  localNonce: bigint;
  remoteNonce: bigint;
  localBalance: bigint;
  remoteBalance: bigint;
  unprocessedCheckpointId: number;
  unprocessedOutflows: bigint;
  unprocessedInflows: bigint;
}

class UserWalletCache {
  cache: Record<number, IUserCacheRecord> = {};

  getUserCached(userId: number): IUserCacheRecord {
    if(!Object.hasOwnProperty.call(this.cache, userId)){
      this.cache[userId] = {
        localNonce: BigInt(0),
        remoteNonce: BigInt(0),
        localBalance: BigInt(0),
        remoteBalance: BigInt(0),
        unprocessedCheckpointId: -1,
        unprocessedOutflows: BigInt(0),
        unprocessedInflows: BigInt(0),
      };
    }
    return this.cache[userId];
  }

  async refreshUserFull(rpc: ICityRPCProvider, userId: number): Promise<{cache: IUserCacheRecord, user: ICityUserState}> {
    const user = await rpc.getUserById(MAX_CHECKPOINT_ID, userId);
    const currentBlock = await rpc.getLatestBlockState();
    const cachedUser = this.getUserCached(userId);
    if(currentBlock.checkpoint_id > cachedUser.unprocessedCheckpointId){
      cachedUser.unprocessedCheckpointId = currentBlock.checkpoint_id;
      cachedUser.unprocessedOutflows = BigInt(0);
      cachedUser.unprocessedInflows = BigInt(0);
      cachedUser.localBalance = cityFelt(user.balance);
    }
    cachedUser.remoteBalance = cityFelt(user.balance);
    cachedUser.remoteNonce = cityFelt(user.nonce);

    if(cachedUser.localNonce < cachedUser.remoteNonce){
      cachedUser.localNonce = cachedUser.remoteNonce;
    }

    return {cache: cachedUser, user};
  }
  async refreshUser(rpc: ICityRPCProvider, userId: number): Promise<IUserCacheRecord> {
    return (await this.refreshUserFull(rpc, userId)).cache;
  }

  incNonce(userId: number): bigint {
    const user = this.getUserCached(userId);
    user.localNonce += BigInt(1);
    return user.localNonce;
  }

  async processTransfer(rpc: ICityRPCProvider, sender: SCNumberLike, recipient: SCNumberLike, amount: SCNumberLike): Promise<bigint> {
    const senderUser = await this.refreshUser(rpc, Number(sender+""));
    const recipientUser = await this.refreshUser(rpc, Number(recipient+""));
    senderUser.localBalance -= cityFelt(amount);
    recipientUser.localBalance += cityFelt(amount);
    return this.incNonce(Number(sender+""));
  }

  async processClaimDeposit(rpc: ICityRPCProvider, user: SCNumberLike, amount: SCNumberLike) {
    const recipientUser = await this.refreshUser(rpc, Number(user+""));
    recipientUser.localBalance += cityFelt(amount)-cityFelt(DEPOSIT_FEE_AMOUNT);
  }
  async processWithdrawal(rpc: ICityRPCProvider, user: SCNumberLike, amount: SCNumberLike): Promise<bigint> {
    const recipientUser = await this.refreshUser(rpc, Number(user+""));
    recipientUser.localBalance -= cityFelt(amount);
    recipientUser.localBalance -= cityFelt(WITHDRAWAL_FEE_AMOUNT);
    return this.incNonce(Number(user+""));
  }


  

  

}


const userWalletCache = new UserWalletCache();

export {
  userWalletCache,
}

export type {
  IUserCacheRecord,
}