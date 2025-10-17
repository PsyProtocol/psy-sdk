import { MultiCoordinatorRpcProvider } from "../coord-edge-rpc";
import { Felt } from "../core";
import { RpcConfig } from "../provider";
import { MultiRealmRpcProvider } from "../realm-edge-rpc";

export class RpcProvider {
  coordinatorRpcProvider: MultiCoordinatorRpcProvider;
  realmRpcProvider: MultiRealmRpcProvider;
  constructor(coordinatorConfig: RpcConfig[], realmConfig: RpcConfig[], userPerRealm: number) {
    this.coordinatorRpcProvider = new MultiCoordinatorRpcProvider(coordinatorConfig);
    this.realmRpcProvider = new MultiRealmRpcProvider(realmConfig, userPerRealm);
  }

  setUserId(userId: number) {
    this.realmRpcProvider.setUserId(userId);
  }

  async getLastClaimCheckpointId(checkpointId: Felt, userId: Felt): Promise<Felt> {
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

  async getPsyBalance(checkpointId: Felt, userId: Felt): Promise<Felt> {
    const leafHash = await this.realmRpcProvider.getRpcProviderByUserId(userId).getUserContractStateTreeLeafHash(checkpointId, userId, 0, 32, 0);

    if (!leafHash || leafHash.length != 64) {
      console.warn("getPsyBalance failed, leafHash.length != 64, leafHash: " + leafHash);
      throw new Error("getPsyBalance failed, leafHash.length != 64");
    }
    console.log("getPsyBalance, leafHash: " + leafHash);
    return parseInt(leafHash?.substring(48, 64), 16);
  }

  async checkTxIsConfirmed(checkpointId: Felt, pkHash: string, txHash: string): Promise<boolean> {
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
}
