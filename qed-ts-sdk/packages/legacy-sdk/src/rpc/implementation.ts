import { DogeNetworkId } from "doge-sdk/dist/types";
import type {
    CityHash,
    ICityUserState,
    CityMerkleProof,
    ICityL1Deposit,
    Hash256,
    ICityL2BlockState,
    Hash160,
    ICityL1Withdrawal,
    QProvingJobDataIDSerializedWrapped,
    TProofValueStoreKV,
    ICityRegisterUserRPCRequest,
    ICityAddWithdrawalRPCRequest,
    ICityClaimDepositRPCRequest,
    ICityTokenTransferRPCRequest,
    ISimpleKVPair,
} from "./baseTypes";
import type { ICityRPCProvider } from "./types";
import { getCityNetworkMagicForNetworkId } from "../action/constants";
import { FetchHTTPClient } from "../http/fetchClient";
import type { IHTTPClient } from "../http/types";

import { QedJSON } from "../utils/json";

class CityRPCProvider implements ICityRPCProvider {
    url: string;
    httpClient: IHTTPClient;
    networkId: DogeNetworkId;
    l2NetworkMagic: string;
    constructor(url: string, httpClient?: IHTTPClient) {
        const tURL = new URL(url);
        this.networkId = (tURL.searchParams.get("networkId") || "doge") as DogeNetworkId;
        this.l2NetworkMagic = getCityNetworkMagicForNetworkId(this.networkId);
        tURL.searchParams.delete("networkId");
        this.url = tURL.toString();
        this.httpClient = httpClient || new FetchHTTPClient();
    }
    getNetworkMagic(): string {
        return this.l2NetworkMagic;
    }
    getDogeNetworkId(): DogeNetworkId {
        return this.networkId;
    }
    async rpc<T>(method: string, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
        const resultBase = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });
        const result = QedJSON.parse(resultBase.body);

        if (result.statusCode >= 400) {
            throw new Error("Error in RPC call: " + resultBase.body);
        } else {
            return result.result as T;
        }
    }
    async rpcMethod<T>(method: string, params: any, id = "1", jsonrpc = "2.0"): Promise<T> {
        const resultBase = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: QedJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });
        const result = QedJSON.parse(resultBase.body);

        if (result.statusCode >= 400) {
            throw new Error("Error in RPC call: " + QedJSON.stringify(resultBase.body));
        } else {
            return result.result as T;
        }
    }
    async getUserTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.rpc("cr_getUserTreeRoot", [checkpoint_id]);
    }
    async getUserIdsForPublicKey(public_key: CityHash): Promise<number[]> {
        return this.rpc("cr_getUserIdsForPublicKey", [public_key]);
    }
    async getUserById(checkpoint_id: number, user_id: number): Promise<ICityUserState> {
        return this.rpc("cr_getUserById", [checkpoint_id, user_id]);
    }
    async getUserMerkleProofById(checkpoint_id: number, user_id: number): Promise<CityMerkleProof> {
        return this.rpc("cr_getUserMerkleProofById", [checkpoint_id, user_id]);
    }
    async getUserTreeLeaf(checkpoint_id: number, leaf_id: number): Promise<CityHash> {
        return this.rpc("cr_getUserTreeLeaf", [checkpoint_id, leaf_id]);
    }
    async getUserTreeLeafMerkleProof(checkpoint_id: number, leaf_id: number): Promise<CityMerkleProof> {
        return this.rpc("cr_getUserTreeLeafMeckleProof", [checkpoint_id, leaf_id]);
    }
    async getDepositTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.rpc("cr_getDepositTreeRoot", [checkpoint_id]);
    }
    async getDepositById(checkpoint_id: number, deposit_id: number): Promise<ICityL1Deposit> {
        return this.rpc("cr_getDepositById", [checkpoint_id, deposit_id]);
    }
    async getDepositsById(checkpoint_id: number, deposit_ids: number[]): Promise<ICityL1Deposit[]> {
        return this.rpc("cr_getDepositsById", [checkpoint_id, deposit_ids]);
    }
    async getDepositByTxid(transaction_id: Hash256): Promise<ICityL1Deposit> {
        return this.rpc("cr_getDepositByTxid", [transaction_id]);
    }
    async getDepositsByTxid(transaction_ids: Hash256[]): Promise<ICityL1Deposit[]> {
        return this.rpc("cr_getDepositsByTxid", [transaction_ids]);
    }
    async getDepositHash(checkpoint_id: number, deposit_id: number): Promise<CityHash> {
        return this.rpc("cr_getDepositHash", [checkpoint_id, deposit_id]);
    }
    async getDepositLeafMerkleProof(checkpoint_id: number, deposit_id: number): Promise<CityMerkleProof> {
        return this.rpc("cr_getDepositLeafMerkleProof", [checkpoint_id, deposit_id]);
    }
    async getBlockState(checkpoint_id: number): Promise<ICityL2BlockState> {
        return this.rpc("cr_getBlockState", [checkpoint_id]);
    }
    getLatestBlockState(): Promise<ICityL2BlockState> {
        return this.rpc("cr_getLatestBlockState", []);
    }
    async getCityRoot(checkpoint_id: number): Promise<CityHash> {
        return this.rpc("cr_getCityRoot", [checkpoint_id]);
    }
    async getCityBlockScript(checkpoint_id: number): Promise<string> {
        return this.rpc("cr_getCityBlockScript", [checkpoint_id]);
    }
    async getCityBlockDepositAddress(checkpoint_id: number): Promise<Hash160> {
        return this.rpc("cr_getCityBlockDepositAddress", [checkpoint_id]);
    }
    async getCityBlockDepositAddressString(checkpoint_id: number): Promise<string> {
        return this.rpc("cr_getCityBlockDepositAddressString", [checkpoint_id]);
    }
    async getWithdrawalTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.rpc("cr_getWithdrawalTreeRoot", [checkpoint_id]);
    }
    async getWithdrawalById(checkpoint_id: number, withdrawal_id: number): Promise<ICityL1Withdrawal> {
        return this.rpc("cr_getWithdrawalById", [checkpoint_id, withdrawal_id]);
    }
    async getWithdrawalsById(checkpoint_id: number, withdrawal_ids: number[]): Promise<ICityL1Withdrawal[]> {
        return this.rpc("cr_getWithdrawalsById", [checkpoint_id, withdrawal_ids]);
    }
    async getWithdrawalHash(checkpoint_id: number, withdrawal_id: number): Promise<CityHash> {
        return this.rpc("cr_getWithdrawalHash", [checkpoint_id, withdrawal_id]);
    }
    async getWithdrawalLeafMerkleProof(checkpoint_id: number, withdrawal_id: number): Promise<CityMerkleProof> {
        return this.rpc("cr_getWithdrawalLeafMerkleProof", [checkpoint_id, withdrawal_id]);
    }
    async getProofStoreValue(key: QProvingJobDataIDSerializedWrapped): Promise<string> {
        return this.rpc("cr_getProofStoreValue", [key]);
    }
    async getProofStoreValues(keys: QProvingJobDataIDSerializedWrapped[]): Promise<TProofValueStoreKV[]> {
        return this.rpc("cr_getProofStoreValues", [keys]);
    }
    async getProofStoreJobWitness(key: QProvingJobDataIDSerializedWrapped): Promise<any> {
        return this.rpc("cr_getProofStoreJobWitness", [key]);
    }
    async getProofStoreJobWitnesses(keys: QProvingJobDataIDSerializedWrapped[]): Promise<ISimpleKVPair<string, any>[]> {
        return this.rpc("cr_getProofStoreJobWitnesses", [keys]);
    }
    async registerUser<F>(req: ICityRegisterUserRPCRequest): Promise<void> {
        return this.rpcMethod("cr_register_user", req.public_key);
    }
    async addWithdrawal(req: ICityAddWithdrawalRPCRequest): Promise<void> {
        return this.rpcMethod("cr_add_withdrawal", req);
    }
    async claimDeposit(req: ICityClaimDepositRPCRequest): Promise<void> {
        return this.rpcMethod("cr_claim_deposit", req);
    }
    async tokenTransfer(req: ICityTokenTransferRPCRequest): Promise<void> {
        return this.rpcMethod("cr_token_transfer", req);
    }

    produceBlock(): Promise<void> {
        return this.rpcMethod("cr_produce_block", null);
    }
}

export { CityRPCProvider };
