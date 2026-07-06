import { ICoordinatorEdgeRpcProvider } from "./types";
import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig, RpcConfig } from "../provider";
import { CheckpointSyncInfo, ContractCodeDefinition, QBCDeployContract, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf, PsyCheckpointSyncInfoCompact, PsyContractLeaf, PsyUserLeaf, PsyBlockState, ZKPublicKeyInfo } from "../types";
/**
 * Enhanced implementation of the Coordinator Edge RPC Provider with caching, retry logic, and multi-provider support
 */
export declare class CoordinatorEdgeRpcProvider extends Provider implements ICoordinatorEdgeRpcProvider {
    private readonly readOnlyMethods;
    constructor(urlOrUrls: string | string[], configOrHttpClient?: ClientConfig | IHTTPClient, httpClient?: IHTTPClient);
    /**
     * Get read-only methods for caching
     */
    protected getReadOnlyMethods(): Set<string>;
    /**
     * Get health check method
     */
    protected getHealthCheckMethod(): string;
    /**
     * Register a user with their ZK public key
     * @param pubKey The ZK public key info
     * @returns A confirmation message
     */
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string>;
    /**
     * Get a user ID from a QHash
     * @param qhash The QHash of the user's public key
     * @returns The user ID
     */
    getUserId(publicKey: QHashOut): Promise<number>;
    /**
     * Deploy a contract
     * @param contract The contract deployment parameters
     * @returns A confirmation message
     */
    deployContract(contract: QBCDeployContract): Promise<string>;
    /**
     * Get the latest checkpoint Id information
     * @returns The latest checkpoint response
     */
    getLatestCheckpointId(): Promise<number>;
    /**
     * Build a new block
     * @returns A confirmation message
     */
    buildBlock(): Promise<string>;
    /**
     * Get checkpoint sync information
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint sync information
     */
    getCheckpointSyncInfo(checkpointId: Felt): Promise<CheckpointSyncInfo>;
    /**
     * Get contract leaf data
     * @param contractId The contract ID
     * @returns Contract leaf data
     */
    getContractLeafData(contractId: Felt): Promise<PsyContractLeaf>;
    /**
     * Get contract leaf data with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract leaf data
     */
    getContractLeafDataF(contractId: Felt): Promise<PsyContractLeaf>;
    /**
     * Get checkpoint leaf data
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint leaf data
     */
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    /**
     * Get checkpoint leaf data with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns Checkpoint leaf data
     */
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    /**
     * Get contract code definition
     * @param contractId The contract ID
     * @returns Contract code definition
     */
    getContractCodeDefinition(contractId: Felt): Promise<ContractCodeDefinition>;
    /**
     * Get contract code definition with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract code definition
     */
    getContractCodeDefinitionF(contractId: Felt): Promise<ContractCodeDefinition>;
    /**
     * Get latest block state
     * @returns block state
     */
    getLatestBlockState(): Promise<PsyBlockState>;
    /**
     * Get block state for a specific checkpoint
     * @param checkpointId The checkpoint ID
     * @returns block state
     */
    getBlockState(checkpointId: Felt): Promise<PsyBlockState>;
    /**
     * Get block state with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns block state
     */
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState>;
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserRegistrationTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserRegistrationTreeLeafHash(checkpointId: Felt, leafIndex: Felt): Promise<QHashOut>;
    getUserRegistrationTreeLeafHashF(checkpointId: Felt, leafIndex: Felt): Promise<QHashOut>;
    getUserRegistrationTreeMerkleProof(checkpointId: Felt, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserRegistrationTreeMerkleProofF(checkpointId: Felt, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: number): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getUserSubTreeMerkleProof(checkpointId: number, rootLevel: number, leafLevel: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeMerkleProof(checkpointId: number, leafLevel: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeCapRoot(checkpointId: number, capLevel: number, capIndex: number): Promise<QHashOut>;
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut>;
    getContractFunctionTreeRoot(checkpointId: number, contractId: number): Promise<QHashOut>;
    getContractFunctionTreeRootF(checkpointId: bigint, contractId: bigint): Promise<QHashOut>;
    getContractFunctionTreeLeafHash(checkpointId: number, contractId: number, functionId: number): Promise<QHashOut>;
    getContractFunctionTreeLeafHashF(checkpointId: bigint, contractId: bigint, functionId: bigint): Promise<QHashOut>;
    getContractFunctionTreeMerkleProof(checkpointId: number, contractId: number, functionId: number): Promise<MerkleProofCore<QHashOut>>;
    getContractFunctionTreeMerkleProofF(checkpointId: bigint, contractId: bigint, functionId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getContractTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getContractTreeLeafHash(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractTreeLeafHashF(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractTreeMerkleProof(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeMerkleProofF(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getDepositTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getDepositTreeLeafHash(checkpointId: Felt, depositId: Felt): Promise<QHashOut>;
    getDepositTreeLeafHashF(checkpointId: Felt, depositId: Felt): Promise<QHashOut>;
    getDepositTreeMerkleProof(checkpointId: Felt, depositId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeMerkleProofF(checkpointId: Felt, depositId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getWithdrawalTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getWithdrawalTreeLeafHash(checkpointId: Felt, withdrawalId: Felt): Promise<QHashOut>;
    getWithdrawalTreeLeafHashF(checkpointId: Felt, withdrawalId: Felt): Promise<QHashOut>;
    getWithdrawalTreeMerkleProof(checkpointId: Felt, withdrawalId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeMerkleProofF(checkpointId: Felt, withdrawalId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots>;
    getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<PsyCheckpointSyncInfoCompact>;
    latestCheckpoint(): Promise<number>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
export declare class MultiCoordinatorRpcProvider implements ICoordinatorEdgeRpcProvider {
    rpcs: Map<number, ICoordinatorEdgeRpcProvider>;
    constructor(coordinatorRpcConfigs: RpcConfig[]);
    getCurrentCoordinatorId(): number;
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string>;
    getUserId(publicKey: QHashOut): Promise<number>;
    deployContract(contract: QBCDeployContract): Promise<string>;
    getLatestCheckpointId(): Promise<number>;
    buildBlock(): Promise<string>;
    getCheckpointSyncInfo(checkpointId: Felt): Promise<CheckpointSyncInfo>;
    getContractLeafData(contractId: Felt): Promise<PsyContractLeaf>;
    getContractLeafDataF(contractId: Felt): Promise<PsyContractLeaf>;
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf>;
    getContractCodeDefinition(contractId: Felt): Promise<ContractCodeDefinition>;
    getContractCodeDefinitionF(contractId: Felt): Promise<ContractCodeDefinition>;
    getLatestBlockState(): Promise<PsyBlockState>;
    getBlockState(checkpointId: Felt): Promise<PsyBlockState>;
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState>;
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserRegistrationTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserRegistrationTreeLeafHash(checkpointId: Felt, leafIndex: number): Promise<QHashOut>;
    getUserRegistrationTreeLeafHashF(checkpointId: Felt, leafIndex: bigint): Promise<QHashOut>;
    getUserRegistrationTreeMerkleProof(checkpointId: Felt, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserRegistrationTreeMerkleProofF(checkpointId: Felt, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getUserSubTreeMerkleProof(checkpointId: Felt, rootLevel: number, leafLevel: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeMerkleProof(checkpointId: Felt, leafLevel: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeCapRoot(checkpointId: Felt, capLevel: number, capIndex: number): Promise<QHashOut>;
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut>;
    getContractFunctionTreeRoot(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractFunctionTreeRootF(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractFunctionTreeLeafHash(checkpointId: Felt, contractId: Felt, functionId: number): Promise<QHashOut>;
    getContractFunctionTreeLeafHashF(checkpointId: Felt, contractId: Felt, functionId: bigint): Promise<QHashOut>;
    getContractFunctionTreeMerkleProof(checkpointId: Felt, contractId: Felt, functionId: number): Promise<MerkleProofCore<QHashOut>>;
    getContractFunctionTreeMerkleProofF(checkpointId: Felt, contractId: Felt, functionId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getContractTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getContractTreeLeafHash(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractTreeLeafHashF(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractTreeMerkleProof(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeMerkleProofF(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getDepositTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getDepositTreeLeafHash(checkpointId: Felt, depositId: number): Promise<QHashOut>;
    getDepositTreeLeafHashF(checkpointId: Felt, depositId: bigint): Promise<QHashOut>;
    getDepositTreeMerkleProof(checkpointId: Felt, depositId: number): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeMerkleProofF(checkpointId: Felt, depositId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getWithdrawalTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getWithdrawalTreeLeafHash(checkpointId: Felt, withdrawalId: number): Promise<QHashOut>;
    getWithdrawalTreeLeafHashF(checkpointId: Felt, withdrawalId: bigint): Promise<QHashOut>;
    getWithdrawalTreeMerkleProof(checkpointId: Felt, withdrawalId: number): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeMerkleProofF(checkpointId: Felt, withdrawalId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots>;
    getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<PsyCheckpointSyncInfoCompact>;
    latestCheckpoint(): Promise<number>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
//# sourceMappingURL=client.d.ts.map