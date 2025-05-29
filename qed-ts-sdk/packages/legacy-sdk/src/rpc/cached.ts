import { DogeNetworkId } from "doge-sdk/dist/types";
import {
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
    ISimpleKVPair,
    ICityRegisterUserRPCRequest,
    ICityAddWithdrawalRPCRequest,
    ICityClaimDepositRPCRequest,
    ICityTokenTransferRPCRequest,
    SCFelt,
} from "./baseTypes";
import { ICityRPCProvider } from "./types";
const MAX_REAL_CHECKPOINT_ID = 999999;
function genCacheKey(method: string, checkpointId: SCFelt, args?: (string | number)[] | (string | number)): string {
    return `c~${method}~${checkpointId}~${Array.isArray(args) ? args.join("~") : args ? args.toString() : ""}`;
}
const ZERO_HASH = "0000000000000000000000000000000000000000000000000000000000000000";

class CityRPCProviderWithCache implements ICityRPCProvider {
    base: ICityRPCProvider;
    cache: Record<string, any> = {};
    constructor(base: ICityRPCProvider) {
        this.base = base;
    }
    clearCache() {
        this.cache = {};
    }
    async getFromCacheOrFallback<T>(
        action: () => Promise<T>,
        isEmpty: (result: T) => boolean,
        method: string,
        checkpointId: SCFelt,
        args?: (string | number)[] | (string | number)
    ): Promise<T> {
        if (checkpointId <= MAX_REAL_CHECKPOINT_ID) {
            const key = genCacheKey(method, checkpointId, args);
            if (Object.prototype.hasOwnProperty.call(this.cache, key)) {
                return this.cache[key];
            } else {
                const result: T = await action();
                if (isEmpty(result)) {
                    return result;
                } else {
                    this.cache[key] = result;
                    return result;
                }
            }
        } else {
            return await action();
        }
    }
    getUserTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.base.getUserTreeRoot(checkpoint_id);
    }
    getUserIdsForPublicKey(public_key: CityHash): Promise<number[]> {
        return this.base.getUserIdsForPublicKey(public_key);
    }
    getUserById(checkpoint_id: number, user_id: number): Promise<ICityUserState> {
        return this.getFromCacheOrFallback<ICityUserState>(
            () => this.base.getUserById(checkpoint_id, user_id),
            (result) => result.public_key === ZERO_HASH,
            "getUserById",
            checkpoint_id,
            user_id
        );
    }
    getUserMerkleProofById(checkpoint_id: number, user_id: number): Promise<CityMerkleProof> {
        return this.base.getUserMerkleProofById(checkpoint_id, user_id);
    }
    getUserTreeLeaf(checkpoint_id: number, leaf_id: number): Promise<CityHash> {
        return this.base.getUserTreeLeaf(checkpoint_id, leaf_id);
    }
    getUserTreeLeafMerkleProof(checkpoint_id: number, leaf_id: number): Promise<CityMerkleProof> {
        return this.base.getUserTreeLeafMerkleProof(checkpoint_id, leaf_id);
    }
    getDepositTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.base.getDepositTreeRoot(checkpoint_id);
    }
    getDepositById(checkpoint_id: number, deposit_id: number): Promise<ICityL1Deposit> {
        return this.getFromCacheOrFallback<ICityL1Deposit>(
            () => this.base.getDepositById(checkpoint_id, deposit_id),
            (result) => result.txid === ZERO_HASH,
            "getDepositById",
            checkpoint_id,
            deposit_id
        );
    }
    async getDepositsById(checkpoint_id: number, deposit_ids: number[]): Promise<ICityL1Deposit[]> {
        let missingIndex = 0;
        const keys = deposit_ids.map((deposit_id, i) => {
            const key = genCacheKey("getDepositById", checkpoint_id, deposit_id);
            const has_cached = Object.prototype.hasOwnProperty.call(this.cache, key);

            return {
                index: i,
                key,
                has_cached,
                checkpoint_id,
                deposit_id,
                missingIndex: has_cached ? -1 : missingIndex++,
            };
        });

        const missingKeys = keys.filter((key) => !key.has_cached);
        const missingIds = missingKeys.map((x) => x.deposit_id);

        const missingDeposits = await this.base.getDepositsById(checkpoint_id, missingIds);
        const deposits = keys.map((x) => {
            if (x.has_cached) {
                return this.cache[x.key];
            } else {
                return missingDeposits[x.missingIndex];
            }
        });

        missingDeposits.forEach((deposit, i) => {
            if (deposit.txid !== ZERO_HASH) {
                this.cache[missingKeys[i].key] = deposit;
            }
        });

        return deposits;
    }
    getDepositByTxid(transaction_id: Hash256): Promise<ICityL1Deposit> {
        return this.base.getDepositByTxid(transaction_id);
    }
    getDepositsByTxid(transaction_ids: Hash256[]): Promise<ICityL1Deposit[]> {
        return this.base.getDepositsByTxid(transaction_ids);
    }
    getDepositHash(checkpoint_id: number, deposit_id: number): Promise<CityHash> {
        return this.base.getDepositHash(checkpoint_id, deposit_id);
    }
    getDepositLeafMerkleProof(checkpoint_id: number, deposit_id: number): Promise<CityMerkleProof> {
        return this.base.getDepositLeafMerkleProof(checkpoint_id, deposit_id);
    }
    getBlockState(checkpoint_id: number): Promise<ICityL2BlockState> {
        return this.base.getBlockState(checkpoint_id);
    }
    getLatestBlockState(): Promise<ICityL2BlockState> {
        return this.base.getLatestBlockState();
    }
    getCityRoot(checkpoint_id: number): Promise<CityHash> {
        return this.base.getCityRoot(checkpoint_id);
    }
    getCityBlockScript(checkpoint_id: number): Promise<string> {
        return this.base.getCityBlockScript(checkpoint_id);
    }
    getCityBlockDepositAddress(checkpoint_id: number): Promise<Hash160> {
        return this.base.getCityBlockDepositAddress(checkpoint_id);
    }
    getCityBlockDepositAddressString(checkpoint_id: number): Promise<string> {
        return this.base.getCityBlockDepositAddressString(checkpoint_id);
    }
    getWithdrawalTreeRoot(checkpoint_id: number): Promise<CityHash> {
        return this.base.getWithdrawalTreeRoot(checkpoint_id);
    }
    getWithdrawalById(checkpoint_id: number, withdrawal_id: number): Promise<ICityL1Withdrawal> {
        return this.getFromCacheOrFallback<ICityL1Withdrawal>(
            () => this.base.getWithdrawalById(checkpoint_id, withdrawal_id),
            (result) => result.value + "" === "0",
            "getWithdrawalById",
            checkpoint_id,
            withdrawal_id
        );
    }
    async getWithdrawalsById(checkpoint_id: number, withdrawal_ids: number[]): Promise<ICityL1Withdrawal[]> {
        let missingIndex = 0;
        const keys = withdrawal_ids.map((withdrawal_id, i) => {
            const key = genCacheKey("getWithdrawalById", checkpoint_id, withdrawal_id);
            const has_cached = Object.prototype.hasOwnProperty.call(this.cache, key);

            return {
                index: i,
                key,
                has_cached,
                checkpoint_id,
                withdrawal_id,
                missingIndex: has_cached ? -1 : missingIndex++,
            };
        });

        const missingKeys = keys.filter((key) => !key.has_cached);
        const missingIds = missingKeys.map((x) => x.withdrawal_id);

        const missingWithdrawals = await this.base.getWithdrawalsById(checkpoint_id, missingIds);
        const withdrawals = keys.map((x) => {
            if (x.has_cached) {
                return this.cache[x.key];
            } else {
                return missingWithdrawals[x.missingIndex];
            }
        });

        missingWithdrawals.forEach((withdrawal, i) => {
            if (withdrawal.value + "" !== "0") {
                this.cache[missingKeys[i].key] = withdrawal;
            }
        });

        return withdrawals;
    }
    getWithdrawalHash(checkpoint_id: number, withdrawal_id: number): Promise<CityHash> {
        return this.base.getWithdrawalHash(checkpoint_id, withdrawal_id);
    }
    getWithdrawalLeafMerkleProof(checkpoint_id: number, withdrawal_id: number): Promise<CityMerkleProof> {
        return this.base.getWithdrawalLeafMerkleProof(checkpoint_id, withdrawal_id);
    }
    getProofStoreValue(key: QProvingJobDataIDSerializedWrapped): Promise<string> {
        return this.base.getProofStoreValue(key);
    }
    getProofStoreValues(keys: QProvingJobDataIDSerializedWrapped[]): Promise<TProofValueStoreKV[]> {
        return this.base.getProofStoreValues(keys);
    }
    getProofStoreJobWitness(key: QProvingJobDataIDSerializedWrapped): Promise<any> {
        return this.getFromCacheOrFallback(
            () => this.base.getProofStoreJobWitness(key),
            (result) => typeof result === "undefined",
            "getProofStoreJobWitness",
            1,
            key
        );
    }
    async getProofStoreJobWitnesses(key: QProvingJobDataIDSerializedWrapped[]): Promise<ISimpleKVPair<string, any>[]> {
        let missingIndex = 0;
        const keys = key.map((k, i) => {
            const key = genCacheKey("getProofStoreJobWitness", 1, k);
            const has_cached = Object.prototype.hasOwnProperty.call(this.cache, key);

            return { index: i, key, has_cached, k, missingIndex: has_cached ? -1 : missingIndex++ };
        });

        const missingKeys = keys.filter((key) => !key.has_cached);
        const missingIds = missingKeys.map((x) => x.k);

        const missingWitnesses = await this.base.getProofStoreJobWitnesses(missingIds);
        if (!(missingWitnesses as any)) {
            throw new Error("one or more witnesses are missing");
        }
        const witnesses = keys.map((x) => {
            if (x.has_cached) {
                return this.cache[x.key];
            } else {
                return missingWitnesses[x.missingIndex];
            }
        });

        missingWitnesses.forEach((w, i) => {
            this.cache[keys[i].key] = w;
        });

        return witnesses;
    }
    registerUser(req: ICityRegisterUserRPCRequest): Promise<void> {
        return this.base.registerUser(req);
    }
    addWithdrawal(req: ICityAddWithdrawalRPCRequest): Promise<void> {
        return this.base.addWithdrawal(req);
    }
    claimDeposit(req: ICityClaimDepositRPCRequest): Promise<void> {
        return this.base.claimDeposit(req);
    }
    tokenTransfer(req: ICityTokenTransferRPCRequest): Promise<void> {
        return this.base.tokenTransfer(req);
    }
    produceBlock(): Promise<void> {
        return this.base.produceBlock();
    }
    getNetworkMagic(): string {
        return this.base.getNetworkMagic();
    }
    getDogeNetworkId(): DogeNetworkId {
        return this.base.getDogeNetworkId();
    }
}

export { CityRPCProviderWithCache };
