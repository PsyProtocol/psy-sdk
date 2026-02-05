import { IRealmEdgeRpcProvider, RealmEdgeRPCCommand } from "./types";
import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig, RpcConfig } from "../provider";
import {
    ProofWithPublicInputs,
    PsyCheckpointGlobalStateRoots,
    PsyCheckpointLeaf,
    PsyBlockState,
    PsyUserLeaf,
    SubmitUserEndCapNonProofInput,
} from "../types";

/**
 * Enhanced RealmEdgeRpcProvider with caching, retry logic, and multi-provider support
 */
export class RealmEdgeRpcProvider extends Provider implements IRealmEdgeRpcProvider {
    // Read-only methods that can be cached
    private readonly readOnlyMethods = new Set<string>([
        RealmEdgeRPCCommand.CheckUserIdInRealm,
        RealmEdgeRPCCommand.GetCheckpointLeafData,
        RealmEdgeRPCCommand.GetCheckpointLeafDataF,
        RealmEdgeRPCCommand.GetLatestBlockState,
        RealmEdgeRPCCommand.GetBlockState,
        RealmEdgeRPCCommand.GetBlockStateF,
        RealmEdgeRPCCommand.GetUserRegistrationTreeRoot,
        RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot,
        RealmEdgeRPCCommand.GetCheckpointTreeRoot,
        RealmEdgeRPCCommand.GetCheckpointTreeRootF,
        RealmEdgeRPCCommand.GetCheckpointTreeLeafHash,
        RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF,
        RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof,
        RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF,
        RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots,
        RealmEdgeRPCCommand.GetUserLeafData,
        RealmEdgeRPCCommand.GetUserLeafDataF,
        RealmEdgeRPCCommand.GetUserContractStateTreeRoot,
        RealmEdgeRPCCommand.GetUserContractStateTreeRootF,
        RealmEdgeRPCCommand.GetUserContractStateTreeLeafHash,
        RealmEdgeRPCCommand.GetUserContractStateTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserContractTreeRoot,
        RealmEdgeRPCCommand.GetUserContractTreeRootF,
        RealmEdgeRPCCommand.GetUserContractTreeLeafHash,
        RealmEdgeRPCCommand.GetUserContractTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserContractTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserTreeRoot,
        RealmEdgeRPCCommand.GetUserTreeRootF,
        RealmEdgeRPCCommand.GetUserTreeLeafHash,
        RealmEdgeRPCCommand.GetUserTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserSubTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserTreeMerkleProofF,
    ]);

    /**
     * Creates a new instance of the Enhanced Realm Edge RPC Provider
     * @param urlOrUrls The URL(s) of the RPC server(s)
     * @param configOrHttpClient Optional enhanced configuration or HTTP client for backward compatibility
     * @param httpClient Optional custom HTTP client
     */
    constructor(
        urlOrUrls: string | string[],
        configOrHttpClient?: ClientConfig | IHTTPClient,
        httpClient?: IHTTPClient
    ) {
        super(urlOrUrls, configOrHttpClient, httpClient);
    }

    setUserId(userId: Felt): void { console.log('setUserId:', userId); }

    /**
     * Get read-only methods for caching
     */
    protected getReadOnlyMethods(): Set<string> {
        return this.readOnlyMethods;
    }

    /**
     * Get health check method
     */
    protected getHealthCheckMethod(): string {
        return RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot;
    }

    // ========== RPC Interface Methods ==========

    // Get RPC provider by user ID
    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider {
        console.log('getRpcProviderByUserId:', userId);
        return this;
    }

    // Check user ID in realm
    async checkUserIdInRealm(userId: Felt): Promise<boolean> {
        return this.rpc(RealmEdgeRPCCommand.CheckUserIdInRealm, [userId]);
    }

    // Submit user end cap
    async submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpc(RealmEdgeRPCCommand.SubmitUserEndCap, [userEcInput, proof]);
    }

    // Get checkpoint leaf data
    async getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    async getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    // Get block state
    async getLatestBlockState(): Promise<PsyBlockState> {
        console.warn("getLatestBlockState");
        return this.rpc(RealmEdgeRPCCommand.GetLatestBlockState, []);
    }

    async getBlockState(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetBlockState, [checkpointId]);
    }

    async getBlockStateF(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetBlockStateF, [checkpointId]);
    }

    // Get user registration tree root
    async getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }

    // Get checkpoint tree roots
    async getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }

    async getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }

    async getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }

    // Get checkpoint tree leaf hash
    async getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHash, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint tree merkle proof
    async getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeMerkleProofF(
        checkpointId: Felt,
        leafCheckpointId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint global state roots
    async getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots, [checkpointId]);
    }

    // Get user leaf data
    async getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafData, [checkpointId, userId]);
    }

    async getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafDataF, [checkpointId, userId]);
    }

    // Get user contract state tree root
    async getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRoot, [checkpointId, userId, contractId]);
    }

    async getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRootF, [checkpointId, userId, contractId]);
    }

    // Get user contract state tree leaf hash
    async getUserContractStateTreeLeafHash(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeLeafHash, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    async getUserContractStateTreeLeafHashF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeLeafHashF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    async getSlotValue(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        slot: Felt
    ): Promise<Felt> {
        const slotValues = await this.getSlotValues(checkpointId, userId, contractId, height, [slot]);
        return slotValues[0];
    }

    async getSlotValues(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        slots: Felt[]
    ): Promise<Felt[]> {
        const slotValues: Felt[] = [];
        for (const slot of slots) {
            const slotIndex = BigInt(slot) / 4n;
            const slotOffset = 3n - BigInt(slot) % 4n;
            const slotHash = await this.getUserContractStateTreeLeafHash(checkpointId, userId, contractId, height, slotIndex);
            const slotValue = parseInt(slotHash?.substring(Number(slotOffset) * 16, Number(slotOffset) * 16 + 16), 16);
            slotValues.push(slotValue);
        }
        return slotValues;
    }

    // Get user contract state tree merkle proof
    async getUserContractStateTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProof, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    async getUserContractStateTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProofF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    // Get user contract tree root
    async getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRoot, [checkpointId, userId]);
    }

    async getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRootF, [checkpointId, userId]);
    }

    // Get user contract tree leaf hash
    async getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHash, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHashF, [checkpointId, userId, contractId]);
    }

    // Get user contract tree merkle proof
    async getUserContractTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProof, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF, [checkpointId, userId, contractId]);
    }

    // Get user tree root
    async getUserTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }

    async getUserTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }

    // Get user tree leaf hash
    async getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHash, [checkpointId, userId]);
    }

    async getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHashF, [checkpointId, userId]);
    }

    // Get user bottom tree merkle proof
    async getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof, [rootLevel, checkpointId, userId]);
    }

    async getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF, [rootLevel, checkpointId, userId]);
    }

    // Get user sub tree merkle proof
    async getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProof, [checkpointId, rootLevel, leafLevel, leafIndex]);
    }

    async getUserSubTreeMerkleProofF(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF, [
            checkpointId,
            rootLevel,
            leafLevel,
            leafIndex,
        ]);
    }

    // Get user tree merkle proof
    async getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProof, [checkpointId, userId]);
    }

    async getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProofF, [checkpointId, userId]);
    }
}

export class MultiRealmRpcProvider implements IRealmEdgeRpcProvider {
    currentUserId: number;
    userPerRealm: number;
    rpcs: Map<number, IRealmEdgeRpcProvider>;
    constructor(realmRpcConfigs: RpcConfig[], userPerRealm: number) {
        this.currentUserId = 0;
        this.userPerRealm = userPerRealm;
        this.rpcs = new Map<number, IRealmEdgeRpcProvider>();

        for (const realmRpcConfig of realmRpcConfigs) {
            this.rpcs.set(realmRpcConfig.id, new RealmEdgeRpcProvider(realmRpcConfig.rpc_url));
        }
    }

    setUserId(userId: number) {
        this.currentUserId = userId;
    }

    getRealmId(userId: number): number {
        return Math.floor(userId / this.userPerRealm);
    }

    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider {
        const realmId = this.getRealmId(Number(userId));
        const provider = this.rpcs.get(realmId);

        if (provider) {
            return provider;
        }
        console.warn(`No RealmEdgeRpcProvider instance available for realmId: ${realmId}`);

        const defaultProvider = this.rpcs.values().next().value;
        if (defaultProvider) {
            return defaultProvider;
        }

        throw new Error("No RealmEdgeRpcProvider instances available");
    }

    checkUserIdInRealm(userId: Felt): Promise<boolean> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.checkUserIdInRealm(userId);
    }
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.submitUserEndCap(userEcInput, proof);
    }
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getCheckpointLeafData(checkpointId);
    }
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getCheckpointLeafDataF(checkpointId);
    }
    getLatestBlockState(): Promise<PsyBlockState> {
        console.warn("getLatestBlockState");
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getLatestBlockState();
    }
    getBlockState(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getBlockState(checkpointId);
    }
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getBlockStateF(checkpointId);
    }
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getUserRegistrationTreeRoot(checkpointId);
    }
    getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getLatestCheckpointTreeRoot();
    }
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getCheckpointTreeRoot(checkpointId);
    }
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getCheckpointTreeRootF(checkpointId);
    }
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getCheckpointTreeLeafHash(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getCheckpointTreeLeafHashF(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getCheckpointTreeMerkleProofF(checkpointId, leafCheckpointId);
    }
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getCheckpointGlobalStateRoots(checkpointId);
    }
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserLeafData(checkpointId, userId);
    }
    getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserLeafDataF(checkpointId, userId);
    }
    getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeRoot(checkpointId, userId, contractId);
    }
    getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeRootF(checkpointId, userId, contractId);
    }
    getUserContractStateTreeLeafHash(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeLeafHash(checkpointId, userId, contractId, height, leafId);
    }
    getUserContractStateTreeLeafHashF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeLeafHashF(checkpointId, userId, contractId, height, leafId);
    }
    async getSlotValue(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        slot: Felt
    ): Promise<Felt> {
        const slotValues = await this.getSlotValues(checkpointId, userId, contractId, height, [slot]);
        return slotValues[0];
    }
    async getSlotValues(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        slots: Felt[]
    ): Promise<Felt[]> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getSlotValues(checkpointId, userId, contractId, height, slots);
    }
    getUserContractStateTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeMerkleProof(checkpointId, userId, contractId, height, leafId);
    }
    getUserContractStateTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractStateTreeMerkleProofF(checkpointId, userId, contractId, height, leafId);
    }
    getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserContractTreeRoot(checkpointId, userId);
    }
    getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserContractTreeRootF(checkpointId, userId);
    }
    getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractTreeLeafHash(checkpointId, userId, contractId);
    }
    getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractTreeLeafHashF(checkpointId, userId, contractId);
    }
    getUserContractTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractTreeMerkleProof(checkpointId, userId, contractId);
    }
    getUserContractTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(Number(userId)))!
            .getUserContractTreeMerkleProofF(checkpointId, userId, contractId);
    }
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getUserTreeRoot(checkpointId);
    }
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getUserTreeRootF(checkpointId);
    }
    getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getUserTreeLeafHash(checkpointId, userId);
    }
    getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getRealmId(this.currentUserId))!.getUserTreeLeafHashF(checkpointId, userId);
    }
    getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getUserBottomTreeMerkleProof(rootLevel, checkpointId, userId);
    }
    getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getUserBottomTreeMerkleProofF(rootLevel, checkpointId, userId);
    }
    getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex);
    }
    getUserSubTreeMerkleProofF(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getRealmId(this.currentUserId))!
            .getUserSubTreeMerkleProofF(checkpointId, rootLevel, leafLevel, leafIndex);
    }
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserTreeMerkleProof(checkpointId, userId);
    }
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getRealmId(Number(userId)))!.getUserTreeMerkleProofF(checkpointId, userId);
    }
}
