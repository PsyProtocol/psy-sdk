import { IRealmEdgeRpcProvider } from "./types";
import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig, RpcConfig } from "../provider";
import { ProofWithPublicInputs, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyBlockState, PsyUserLeaf, SubmitUserEndCapNonProofInput } from "../types";
/**
 * Enhanced RealmEdgeRpcProvider with caching, retry logic, and multi-provider support
 */
export declare class RealmEdgeRpcProvider extends Provider implements IRealmEdgeRpcProvider {
    private readonly readOnlyMethods;
    /**
     * Creates a new instance of the Enhanced Realm Edge RPC Provider
     * @param urlOrUrls The URL(s) of the RPC server(s)
     * @param configOrHttpClient Optional enhanced configuration or HTTP client for backward compatibility
     * @param httpClient Optional custom HTTP client
     */
    constructor(urlOrUrls: string | string[], configOrHttpClient?: ClientConfig | IHTTPClient, httpClient?: IHTTPClient);
    setUserId(userId: Felt): void;
    /**
     * Get read-only methods for caching
     */
    protected getReadOnlyMethods(): Set<string>;
    /**
     * Get health check method
     */
    protected getHealthCheckMethod(): string;
    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider;
    checkUserIdInRealm(userId: Felt): Promise<boolean>;
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string>;
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getLatestBlockState(): Promise<PsyBlockState>;
    getBlockState(checkpointId: Felt): Promise<PsyBlockState>;
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState>;
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getSlotValue(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slot: Felt): Promise<Felt>;
    getSlotValues(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slots: Felt[]): Promise<Felt[]>;
    getUserContractStateTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractStateTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserBottomTreeMerkleProof(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserBottomTreeMerkleProofF(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProof(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProofF(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
export declare class MultiRealmRpcProvider implements IRealmEdgeRpcProvider {
    currentUserId: number;
    userPerRealm: number;
    rpcs: Map<number, IRealmEdgeRpcProvider>;
    constructor(realmRpcConfigs: RpcConfig[], userPerRealm: number);
    setUserId(userId: number): void;
    getRealmId(userId: number): number;
    getRpcProviderByUserId(userId: Felt): IRealmEdgeRpcProvider;
    checkUserIdInRealm(userId: Felt): Promise<boolean>;
    submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string>;
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getLatestBlockState(): Promise<PsyBlockState>;
    getBlockState(checkpointId: Felt): Promise<PsyBlockState>;
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState>;
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserLeafDataF(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserContractStateTreeRoot(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeRootF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getUserContractStateTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<QHashOut>;
    getSlotValue(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slot: Felt): Promise<Felt>;
    getSlotValues(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, slots: Felt[]): Promise<Felt[]>;
    getUserContractStateTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractStateTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt, height: number, leafId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeRoot(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeRootF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHash(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeLeafHashF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<QHashOut>;
    getUserContractTreeMerkleProof(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserContractTreeMerkleProofF(checkpointId: Felt, userId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeLeafHash(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserTreeLeafHashF(checkpointId: Felt, userId: Felt): Promise<QHashOut>;
    getUserBottomTreeMerkleProof(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserBottomTreeMerkleProofF(rootLevel: number, checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProof(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: bigint | number): Promise<MerkleProofCore<QHashOut>>;
    getUserSubTreeMerkleProofF(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
//# sourceMappingURL=client.d.ts.map