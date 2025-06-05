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

/**
 * Enhanced RealmEdgeRpcProvider with caching, retry logic, and multi-provider support
 */
export class RealmEdgeRpcProvider extends Provider implements IRealmEdgeRpcProvider {
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
        return this.rpc(RealmEdgeRPCCommand.CheckUserIdInRealm, [userId]);
    }

    // Submit user end cap
    async submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpc(RealmEdgeRPCCommand.SubmitUserEndCap, [userEcInput, proof]);
    }

    // Get checkpoint leaf data
    async getCheckpointLeafData(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    async getCheckpointLeafDataF(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    // Get L2 block state
    async getLatestL2BlockState(): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestL2BlockState, []);
    }

    async getL2BlockState(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockState, [checkpointId]);
    }

    async getL2BlockStateF(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockStateF, [checkpointId]);
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
    async getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<QEDCheckpointGlobalStateRoots> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots, [checkpointId]);
    }

    // Get user leaf data
    async getUserLeafData(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafData, [checkpointId, userId]);
    }

    async getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf> {
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
