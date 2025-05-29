import {
    IRealmEdgeRpcProvider,
    MerkleProofCore,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDL2BlockState,
    QEDUserLeaf,
    QHashOut,
    RealmEdgeRPCCommand,
    SubmitUserEndCapNonProofInput,
} from "./types";
import { Provider, ClientConfig } from "../common/provider";
import { IHTTPClient } from "../http/types";
import { ProofWithPublicInputs } from "../rpc/plonkTypes";

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
    async checkUserIdInRealm(userId: bigint | number): Promise<boolean> {
        return this.rpc(RealmEdgeRPCCommand.CheckUserIdInRealm, [userId]);
    }

    // Submit user end cap
    async submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpc(RealmEdgeRPCCommand.SubmitUserEndCap, [userEcInput, proof]);
    }

    // Get checkpoint leaf data
    async getCheckpointLeafData(checkpointId: bigint | number): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    async getCheckpointLeafDataF(checkpointId: bigint): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    // Get L2 block state
    async getLatestL2BlockState(): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestL2BlockState, []);
    }

    async getL2BlockState(checkpointId: bigint | number): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockState, [checkpointId]);
    }

    async getL2BlockStateF(checkpointId: bigint): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockStateF, [checkpointId]);
    }

    // Get user registration tree root
    async getUserRegistrationTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }

    // Get checkpoint tree roots
    async getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }

    async getCheckpointTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }

    async getCheckpointTreeRootF(checkpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }

    // Get checkpoint tree leaf hash
    async getCheckpointTreeLeafHash(
        checkpointId: bigint | number,
        leafCheckpointId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHash, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeLeafHashF(checkpointId: bigint, leafCheckpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint tree merkle proof
    async getCheckpointTreeMerkleProof(
        checkpointId: bigint | number,
        leafCheckpointId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeMerkleProofF(
        checkpointId: bigint,
        leafCheckpointId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint global state roots
    async getCheckpointGlobalStateRoots(checkpointId: bigint | number): Promise<QEDCheckpointGlobalStateRoots> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots, [checkpointId]);
    }

    // Get user leaf data
    async getUserLeafData(checkpointId: bigint | number, userId: bigint | number): Promise<QEDUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafData, [checkpointId, userId]);
    }

    async getUserLeafDataF(checkpointId: bigint, userId: bigint): Promise<QEDUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafDataF, [checkpointId, userId]);
    }

    // Get user contract state tree root
    async getUserContractStateTreeRoot(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRoot, [checkpointId, userId, contractId]);
    }

    async getUserContractStateTreeRootF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRootF, [checkpointId, userId, contractId]);
    }

    // Get user contract state tree leaf hash
    async getUserContractStateTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
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
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
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
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
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
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
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
    async getUserContractTreeRoot(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRoot, [checkpointId, userId]);
    }

    async getUserContractTreeRootF(checkpointId: bigint, userId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRootF, [checkpointId, userId]);
    }

    // Get user contract tree leaf hash
    async getUserContractTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHash, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeLeafHashF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHashF, [checkpointId, userId, contractId]);
    }

    // Get user contract tree merkle proof
    async getUserContractTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProof, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeMerkleProofF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF, [checkpointId, userId, contractId]);
    }

    // Get user tree root
    async getUserTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }

    async getUserTreeRootF(checkpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }

    // Get user tree leaf hash
    async getUserTreeLeafHash(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHash, [checkpointId, userId]);
    }

    async getUserTreeLeafHashF(checkpointId: bigint, userId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHashF, [checkpointId, userId]);
    }

    // Get user bottom tree merkle proof
    async getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: bigint | number,
        userId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof, [rootLevel, checkpointId, userId]);
    }

    async getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: bigint,
        userId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF, [rootLevel, checkpointId, userId]);
    }

    // Get user sub tree merkle proof
    async getUserSubTreeMerkleProof(
        checkpointId: bigint | number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProof, [checkpointId, rootLevel, leafLevel, leafIndex]);
    }

    async getUserSubTreeMerkleProofF(
        checkpointId: bigint,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF, [
            checkpointId,
            rootLevel,
            leafLevel,
            leafIndex,
        ]);
    }

    // Get user tree merkle proof
    async getUserTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProof, [checkpointId, userId]);
    }

    async getUserTreeMerkleProofF(checkpointId: bigint, userId: bigint): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProofF, [checkpointId, userId]);
    }
}
