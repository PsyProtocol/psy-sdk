import { IRealmEdgeRpcProvider, RealmEdgeRPCCommand } from "./types";
import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig } from "../provider";
import {
    ProofWithPublicInputs,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDL2BlockState,
    QEDUserLeaf,
    SubmitUserEndCapNonProofInput,
} from "../types";

const USER_PER_REALM = 2048;
/**
 * Enhanced RealmEdgeRpcProvider with caching, retry logic, and multi-provider support
 */
export class RealmEdgeRpcProvider extends Provider implements IRealmEdgeRpcProvider {
    private userId: Felt;
    // Read-only methods that can be cached
    private readonly readOnlyMethods = new Set<string>([
        RealmEdgeRPCCommand.CheckUserIdInRealm,
        RealmEdgeRPCCommand.GetCheckpointLeafData,
        RealmEdgeRPCCommand.GetCheckpointLeafDataF,
        RealmEdgeRPCCommand.GetLatestL2BlockState,
        RealmEdgeRPCCommand.GetL2BlockState,
        RealmEdgeRPCCommand.GetL2BlockStateF,
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
        this.userId = 0;
    }

    setUserId(userId: Felt): void {
        this.userId = userId;
    }

    /**
     * Select realm according to user ID
     */
    selectRealmUrl(userId: Felt): string {
        var realm_id = Math.floor(Number(userId) / USER_PER_REALM);
        if (realm_id >= this.urls.length) {
            realm_id = 0;
        }
        return this.urls[realm_id];
    }

    protected async rpc_with_user_id<T>(
        userId: Felt,
        method: string,
        params: unknown,
        id = "1",
        jsonrpc = "2.0",
        headers?: Record<string, string>
    ): Promise<T> {
        return this.rpc_with_url<T>(this.selectRealmUrl(userId), method, params, id, jsonrpc, headers);
    }

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

    // Check user ID in realm
    async checkUserIdInRealm(userId: Felt): Promise<boolean> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.CheckUserIdInRealm, [userId]);
    }

    // Submit user end cap
    async submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.SubmitUserEndCap, [userEcInput, proof]);
    }

    // Get checkpoint leaf data
    async getCheckpointLeafData(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    async getCheckpointLeafDataF(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    // Get L2 block state
    async getLatestL2BlockState(): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestL2BlockState, []);
    }

    async getL2BlockState(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetL2BlockState, [checkpointId]);
    }

    async getL2BlockStateF(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetL2BlockStateF, [checkpointId]);
    }

    // Get user registration tree root
    async getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }

    // Get checkpoint tree roots
    async getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }

    async getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }

    async getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }

    // Get checkpoint tree leaf hash
    async getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeLeafHash, [
            checkpointId,
            leafCheckpointId,
        ]);
    }

    async getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF, [
            checkpointId,
            leafCheckpointId,
        ]);
    }

    // Get checkpoint tree merkle proof
    async getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof, [
            checkpointId,
            leafCheckpointId,
        ]);
    }

    async getCheckpointTreeMerkleProofF(
        checkpointId: Felt,
        leafCheckpointId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF, [
            checkpointId,
            leafCheckpointId,
        ]);
    }

    // Get checkpoint global state roots
    async getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<QEDCheckpointGlobalStateRoots> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots, [checkpointId]);
    }

    // Get user leaf data
    async getUserLeafData(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserLeafData, [checkpointId, userId]);
    }

    async getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserLeafDataF, [checkpointId, userId]);
    }

    // Get user contract state tree root
    async getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeRoot, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    async getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeRootF, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    // Get user contract state tree leaf hash
    async getUserContractStateTreeLeafHash(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeLeafHash, [
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
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeLeafHashF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    // Get user contract state tree merkle proof
    async getUserContractStateTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt,
        height: number,
        leafId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProof, [
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
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProofF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    // Get user contract tree root
    async getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractTreeRoot, [checkpointId, userId]);
    }

    async getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRootF, [checkpointId, userId]);
    }

    // Get user contract tree leaf hash
    async getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractTreeLeafHash, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    async getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractTreeLeafHashF, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    // Get user contract tree merkle proof
    async getUserContractTreeMerkleProof(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractTreeMerkleProof, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    async getUserContractTreeMerkleProofF(
        checkpointId: Felt,
        userId: Felt,
        contractId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF, [
            checkpointId,
            userId,
            contractId,
        ]);
    }

    // Get user tree root
    async getUserTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }

    async getUserTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }

    // Get user tree leaf hash
    async getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserTreeLeafHash, [checkpointId, userId]);
    }

    async getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserTreeLeafHashF, [checkpointId, userId]);
    }

    // Get user bottom tree merkle proof
    async getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof, [
            rootLevel,
            checkpointId,
            userId,
        ]);
    }

    async getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: Felt,
        userId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF, [
            rootLevel,
            checkpointId,
            userId,
        ]);
    }

    // Get user sub tree merkle proof
    async getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetUserSubTreeMerkleProof, [
            checkpointId,
            rootLevel,
            leafLevel,
            leafIndex,
        ]);
    }

    async getUserSubTreeMerkleProofF(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(this.userId, RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF, [
            checkpointId,
            rootLevel,
            leafLevel,
            leafIndex,
        ]);
    }

    // Get user tree merkle proof
    async getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserTreeMerkleProof, [checkpointId, userId]);
    }

    async getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc_with_user_id(userId, RealmEdgeRPCCommand.GetUserTreeMerkleProofF, [checkpointId, userId]);
    }
}
