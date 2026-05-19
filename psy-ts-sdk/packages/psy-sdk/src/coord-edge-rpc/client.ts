import { CoordinatorEdgeRPCCommand, ICoordinatorEdgeRpcProvider } from "./types";

import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig, RpcConfig } from "../provider";
import {
    CheckpointSyncInfo,
    ContractCodeDefinition,
    QBCDeployContract,
    PsyCheckpointGlobalStateRoots,
    PsyCheckpointLeaf,
    PsyCheckpointSyncInfoCompact,
    PsyContractLeaf,
    PsyUserLeaf,
    PsyBlockState,
    ZKPublicKeyInfo,
} from "../types";

/**
 * Enhanced implementation of the Coordinator Edge RPC Provider with caching, retry logic, and multi-provider support
 */
export class CoordinatorEdgeRpcProvider extends Provider implements ICoordinatorEdgeRpcProvider {
    // Read-only methods that can be cached
    private readonly readOnlyMethods = new Set<string>([
        CoordinatorEdgeRPCCommand.GetUserId,
        CoordinatorEdgeRPCCommand.GetLatestCheckpointId,
        CoordinatorEdgeRPCCommand.GetCheckpointSyncInfo,
        CoordinatorEdgeRPCCommand.GetContractLeafData,
        CoordinatorEdgeRPCCommand.GetContractLeafDataF,
        CoordinatorEdgeRPCCommand.GetCheckpointLeafData,
        CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF,
        CoordinatorEdgeRPCCommand.GetContractCodeDefinition,
        CoordinatorEdgeRPCCommand.GetContractCodeDefinitionF,
        CoordinatorEdgeRPCCommand.GetLatestBlockState,
        CoordinatorEdgeRPCCommand.GetBlockState,
        CoordinatorEdgeRPCCommand.GetBlockStateF,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRoot,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRootF,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetUserTreeRoot,
        CoordinatorEdgeRPCCommand.GetUserTreeRootF,
        CoordinatorEdgeRPCCommand.GetUserSubTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetUserTopTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetUserTopTreeCapRoot,
        CoordinatorEdgeRPCCommand.GetUserLatestTopTreeCapRoot,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeRoot,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeRootF,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetContractTreeRoot,
        CoordinatorEdgeRPCCommand.GetContractTreeRootF,
        CoordinatorEdgeRPCCommand.GetContractTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetContractTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetContractTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetContractTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetDepositTreeRoot,
        CoordinatorEdgeRPCCommand.GetDepositTreeRootF,
        CoordinatorEdgeRPCCommand.GetDepositTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetDepositTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeRoot,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeRootF,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetLatestCheckpointTreeRoot,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeRoot,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeRootF,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHash,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHashF,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProofF,
        CoordinatorEdgeRPCCommand.GetCheckpointGlobalStateRoots,
        CoordinatorEdgeRPCCommand.GetCheckpointSyncInfoCompact,
        CoordinatorEdgeRPCCommand.LatestCheckpoint,
        CoordinatorEdgeRPCCommand.GetUserLeafData,
        CoordinatorEdgeRPCCommand.GetUserTreeMerkleProof,
        CoordinatorEdgeRPCCommand.GetUserTreeMerkleProofF,
    ]);

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
        return CoordinatorEdgeRPCCommand.GetLatestCheckpointId;
    }

    /**
     * Register a user with their ZK public key
     * @param pubKey The ZK public key info
     * @returns A confirmation message
     */
    async registerUser(pubKey: ZKPublicKeyInfo): Promise<string> {
        return this.rpc<string>(CoordinatorEdgeRPCCommand.RegisterUser, [pubKey]);
    }

    /**
     * Get a user ID from a QHash
     * @param qhash The QHash of the user's public key
     * @returns The user ID
     */
    async getUserId(publicKey: QHashOut): Promise<number> {
        const result = await this.rpc<number[]>(CoordinatorEdgeRPCCommand.GetUserId, { public_key: publicKey, start_user_id: 0, count: 64 });
        if (result.length === 0) {
            throw new Error("No user ID found for the given public key");
        }
        return result[0];
    }

    /**
     * Deploy a contract
     * @param contract The contract deployment parameters
     * @returns A confirmation message
     */
    async deployContract(contract: QBCDeployContract): Promise<string> {
        return this.rpc<string>(CoordinatorEdgeRPCCommand.DeployContract, [contract]);
    }

    /**
     * Get the latest checkpoint Id information
     * @returns The latest checkpoint response
     */
    async getLatestCheckpointId(): Promise<number> {
        return this.rpc<number>(CoordinatorEdgeRPCCommand.GetLatestCheckpointId, []);
    }

    /**
     * Build a new block
     * @returns A confirmation message
     */
    async buildBlock(): Promise<string> {
        return this.rpc<string>(CoordinatorEdgeRPCCommand.BuildBlock, []);
    }

    /**
     * Get checkpoint sync information
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint sync information
     */
    async getCheckpointSyncInfo(checkpointId: Felt): Promise<CheckpointSyncInfo> {
        return this.rpc<CheckpointSyncInfo>(CoordinatorEdgeRPCCommand.GetCheckpointSyncInfo, [checkpointId]);
    }

    /**
     * Get contract leaf data
     * @param contractId The contract ID
     * @returns Contract leaf data
     */
    async getContractLeafData(contractId: Felt): Promise<PsyContractLeaf> {
        return this.rpc<PsyContractLeaf>(CoordinatorEdgeRPCCommand.GetContractLeafData, [contractId]);
    }

    /**
     * Get contract leaf data with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract leaf data
     */
    async getContractLeafDataF(contractId: Felt): Promise<PsyContractLeaf> {
        return this.rpc<PsyContractLeaf>(CoordinatorEdgeRPCCommand.GetContractLeafDataF, [contractId]);
    }

    /**
     * Get checkpoint leaf data
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpc<PsyCheckpointLeaf>(CoordinatorEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    /**
     * Get checkpoint leaf data with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpc<PsyCheckpointLeaf>(CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    /**
     * Get contract code definition
     * @param contractId The contract ID
     * @returns Contract code definition
     */
    async getContractCodeDefinition(contractId: Felt): Promise<ContractCodeDefinition> {
        return this.rpc<ContractCodeDefinition>(CoordinatorEdgeRPCCommand.GetContractCodeDefinition, [contractId]);
    }

    /**
     * Get contract code definition with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract code definition
     */
    async getContractCodeDefinitionF(contractId: Felt): Promise<ContractCodeDefinition> {
        return this.rpc<ContractCodeDefinition>(CoordinatorEdgeRPCCommand.GetContractCodeDefinitionF, [contractId]);
    }

    /**
     * Get latest block state
     * @returns block state
     */
    async getLatestBlockState(): Promise<PsyBlockState> {
        return this.rpc<PsyBlockState>(CoordinatorEdgeRPCCommand.GetLatestBlockState, []);
    }

    /**
     * Get block state for a specific checkpoint
     * @param checkpointId The checkpoint ID
     * @returns block state
     */
    async getBlockState(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpc<PsyBlockState>(CoordinatorEdgeRPCCommand.GetBlockState, [checkpointId]);
    }

    /**
     * Get block state with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns block state
     */
    async getBlockStateF(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpc<PsyBlockState>(CoordinatorEdgeRPCCommand.GetBlockStateF, [checkpointId]);
    }

    // User Registration Tree methods
    async getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }

    async getUserRegistrationTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRootF, [checkpointId]);
    }

    async getUserRegistrationTreeLeafHash(checkpointId: Felt, leafIndex: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHash, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }

    async getUserRegistrationTreeLeafHashF(checkpointId: Felt, leafIndex: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHashF, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }

    async getUserRegistrationTreeMerkleProof(checkpointId: Felt, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProof, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }

    async getUserRegistrationTreeMerkleProofF(checkpointId: Felt, leafIndex: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }

    // User Tree methods
    async getUserTreeRoot(checkpointId: number): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }

    async getUserTreeRootF(checkpointId: bigint): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }

    async getUserSubTreeMerkleProof(
        checkpointId: number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserSubTreeMerkleProof, {
            checkpoint_id: checkpointId,
            root_level: rootLevel,
            leaf_level: leafLevel,
            leaf_index: leafIndex,
        });
    }

    async getUserTopTreeMerkleProof(
        checkpointId: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserTopTreeMerkleProof, [
            checkpointId,
            leafLevel,
            leafIndex,
        ]);
    }

    async getUserTopTreeCapRoot(checkpointId: number, capLevel: number, capIndex: number): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserTopTreeCapRoot, [checkpointId, capLevel, capIndex]);
    }

    async getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetUserLatestTopTreeCapRoot, [capLevel, capIndex]);
    }

    // Contract Function Tree methods
    async getContractFunctionTreeRoot(checkpointId: number, contractId: number): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeRoot, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    async getContractFunctionTreeRootF(checkpointId: bigint, contractId: bigint): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeRootF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    async getContractFunctionTreeLeafHash(
        checkpointId: number,
        contractId: number,
        functionId: number
    ): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHash, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }

    async getContractFunctionTreeLeafHashF(
        checkpointId: bigint,
        contractId: bigint,
        functionId: bigint
    ): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHashF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }

    async getContractFunctionTreeMerkleProof(
        checkpointId: number,
        contractId: number,
        functionId: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProof, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }

    async getContractFunctionTreeMerkleProofF(
        checkpointId: bigint,
        contractId: bigint,
        functionId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }

    // Contract Tree methods
    async getContractTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractTreeRoot, [checkpointId]);
    }

    async getContractTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractTreeRootF, [checkpointId]);
    }

    async getContractTreeLeafHash(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractTreeLeafHash, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    async getContractTreeLeafHashF(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetContractTreeLeafHashF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    async getContractTreeMerkleProof(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetContractTreeMerkleProof, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    async getContractTreeMerkleProofF(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetContractTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }

    // Deposit Tree methods
    async getDepositTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetDepositTreeRoot, [checkpointId]);
    }

    async getDepositTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetDepositTreeRootF, [checkpointId]);
    }

    async getDepositTreeLeafHash(checkpointId: Felt, depositId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetDepositTreeLeafHash, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }

    async getDepositTreeLeafHashF(checkpointId: Felt, depositId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetDepositTreeLeafHashF, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }

    async getDepositTreeMerkleProof(checkpointId: Felt, depositId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProof, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }

    async getDepositTreeMerkleProofF(checkpointId: Felt, depositId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }

    // Withdrawal Tree methods
    async getWithdrawalTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeRoot, [checkpointId]);
    }

    async getWithdrawalTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeRootF, [checkpointId]);
    }

    async getWithdrawalTreeLeafHash(checkpointId: Felt, withdrawalId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHash, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }

    async getWithdrawalTreeLeafHashF(checkpointId: Felt, withdrawalId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHashF, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }

    async getWithdrawalTreeMerkleProof(checkpointId: Felt, withdrawalId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProof, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }

    async getWithdrawalTreeMerkleProofF(checkpointId: Felt, withdrawalId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }

    // Checkpoint Tree methods
    async getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }

    async getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }

    async getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }

    async getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHash, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }

    async getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpc<QHashOut>(CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHashF, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }

    async getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProof, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }

    async getCheckpointTreeMerkleProofF(
        checkpointId: Felt,
        leafCheckpointId: Felt
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }

    // Global state and checkpoint info methods
    async getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots> {
        return this.rpc<PsyCheckpointGlobalStateRoots>(CoordinatorEdgeRPCCommand.GetCheckpointGlobalStateRoots, [
            checkpointId,
        ]);
    }

    async getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<PsyCheckpointSyncInfoCompact> {
        return this.rpc<PsyCheckpointSyncInfoCompact>(
            CoordinatorEdgeRPCCommand.GetCheckpointSyncInfoCompact,
            checkpointId
        );
    }

    async latestCheckpoint(): Promise<number> {
        return this.rpc<number>(CoordinatorEdgeRPCCommand.LatestCheckpoint, []);
    }

    // User data methods
    async getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpc<PsyUserLeaf>(CoordinatorEdgeRPCCommand.GetUserLeafData, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }

    async getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserTreeMerkleProof, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }

    async getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc<MerkleProofCore<QHashOut>>(CoordinatorEdgeRPCCommand.GetUserTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }
}
export class MultiCoordinatorRpcProvider implements ICoordinatorEdgeRpcProvider {
    rpcs: Map<number, ICoordinatorEdgeRpcProvider>;
    constructor(coordinatorRpcConfigs: RpcConfig[]) {
        this.rpcs = new Map<number, ICoordinatorEdgeRpcProvider>();
        for (const coordinatorRpcConfig of coordinatorRpcConfigs) {
            this.rpcs.set(coordinatorRpcConfig.id, new CoordinatorEdgeRpcProvider(coordinatorRpcConfig.rpc_url));
        }
    }

    getCurrentCoordinatorId(): number {
        return 0;
    }
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.registerUser(pubKey);
    }
    getUserId(publicKey: QHashOut): Promise<number> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserId(publicKey);
    }
    deployContract(contract: QBCDeployContract): Promise<string> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.deployContract(contract);
    }
    getLatestCheckpointId(): Promise<number> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getLatestCheckpointId();
    }
    buildBlock(): Promise<string> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.buildBlock();
    }
    getCheckpointSyncInfo(checkpointId: Felt): Promise<CheckpointSyncInfo> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointSyncInfo(checkpointId);
    }
    getContractLeafData(contractId: Felt): Promise<PsyContractLeaf> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractLeafData(contractId);
    }
    getContractLeafDataF(contractId: Felt): Promise<PsyContractLeaf> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractLeafDataF(contractId);
    }
    getCheckpointLeafData(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointLeafData(checkpointId);
    }
    getCheckpointLeafDataF(checkpointId: Felt): Promise<PsyCheckpointLeaf> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointLeafDataF(checkpointId);
    }
    getContractCodeDefinition(contractId: Felt): Promise<ContractCodeDefinition> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractCodeDefinition(contractId);
    }
    getContractCodeDefinitionF(contractId: Felt): Promise<ContractCodeDefinition> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractCodeDefinitionF(contractId);
    }
    getLatestBlockState(): Promise<PsyBlockState> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getLatestBlockState();
    }
    getBlockState(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getBlockState(checkpointId);
    }
    getBlockStateF(checkpointId: Felt): Promise<PsyBlockState> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getBlockStateF(checkpointId);
    }
    getUserRegistrationTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserRegistrationTreeRoot(checkpointId);
    }
    getUserRegistrationTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserRegistrationTreeRootF(checkpointId);
    }
    getUserRegistrationTreeLeafHash(checkpointId: Felt, leafIndex: number): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserRegistrationTreeLeafHash(checkpointId, leafIndex);
    }
    getUserRegistrationTreeLeafHashF(checkpointId: Felt, leafIndex: bigint): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserRegistrationTreeLeafHashF(checkpointId, leafIndex);
    }
    getUserRegistrationTreeMerkleProof(checkpointId: Felt, leafIndex: number): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getUserRegistrationTreeMerkleProof(checkpointId, leafIndex);
    }
    getUserRegistrationTreeMerkleProofF(checkpointId: Felt, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getUserRegistrationTreeMerkleProofF(checkpointId, leafIndex);
    }
    getUserTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserTreeRoot(checkpointId);
    }
    getUserTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserTreeRootF(checkpointId);
    }
    getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex);
    }
    getUserTopTreeMerkleProof(
        checkpointId: Felt,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getUserTopTreeMerkleProof(checkpointId, leafLevel, leafIndex);
    }
    getUserTopTreeCapRoot(checkpointId: Felt, capLevel: number, capIndex: number): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserTopTreeCapRoot(checkpointId, capLevel, capIndex);
    }
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserLatestTopTreeCapRoot(capLevel, capIndex);
    }
    getContractFunctionTreeRoot(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractFunctionTreeRoot(checkpointId, contractId);
    }
    getContractFunctionTreeRootF(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractFunctionTreeRootF(checkpointId, contractId);
    }
    getContractFunctionTreeLeafHash(checkpointId: Felt, contractId: Felt, functionId: number): Promise<QHashOut> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getContractFunctionTreeLeafHash(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeLeafHashF(checkpointId: Felt, contractId: Felt, functionId: bigint): Promise<QHashOut> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getContractFunctionTreeLeafHashF(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeMerkleProof(
        checkpointId: Felt,
        contractId: Felt,
        functionId: number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getContractFunctionTreeMerkleProof(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeMerkleProofF(
        checkpointId: Felt,
        contractId: Felt,
        functionId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getContractFunctionTreeMerkleProofF(checkpointId, contractId, functionId);
    }
    getContractTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeRoot(checkpointId);
    }
    getContractTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeRootF(checkpointId);
    }
    getContractTreeLeafHash(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeLeafHash(checkpointId, contractId);
    }
    getContractTreeLeafHashF(checkpointId: Felt, contractId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeLeafHashF(checkpointId, contractId);
    }
    getContractTreeMerkleProof(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeMerkleProof(checkpointId, contractId);
    }
    getContractTreeMerkleProofF(checkpointId: Felt, contractId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getContractTreeMerkleProofF(checkpointId, contractId);
    }
    getDepositTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeRoot(checkpointId);
    }
    getDepositTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeRootF(checkpointId);
    }
    getDepositTreeLeafHash(checkpointId: Felt, depositId: number): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeLeafHash(checkpointId, depositId);
    }
    getDepositTreeLeafHashF(checkpointId: Felt, depositId: bigint): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeLeafHashF(checkpointId, depositId);
    }
    getDepositTreeMerkleProof(checkpointId: Felt, depositId: number): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeMerkleProof(checkpointId, depositId);
    }
    getDepositTreeMerkleProofF(checkpointId: Felt, depositId: bigint): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getDepositTreeMerkleProofF(checkpointId, depositId);
    }
    getWithdrawalTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeRoot(checkpointId);
    }
    getWithdrawalTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeRootF(checkpointId);
    }
    getWithdrawalTreeLeafHash(checkpointId: Felt, withdrawalId: number): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeLeafHash(checkpointId, withdrawalId);
    }
    getWithdrawalTreeLeafHashF(checkpointId: Felt, withdrawalId: bigint): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeLeafHashF(checkpointId, withdrawalId);
    }
    getWithdrawalTreeMerkleProof(checkpointId: Felt, withdrawalId: number): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeMerkleProof(checkpointId, withdrawalId);
    }
    getWithdrawalTreeMerkleProofF(checkpointId: Felt, withdrawalId: bigint): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getWithdrawalTreeMerkleProofF(checkpointId, withdrawalId);
    }
    getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getLatestCheckpointTreeRoot();
    }
    getCheckpointTreeRoot(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointTreeRoot(checkpointId);
    }
    getCheckpointTreeRootF(checkpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointTreeRootF(checkpointId);
    }
    getCheckpointTreeLeafHash(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointTreeLeafHash(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeLeafHashF(checkpointId: Felt, leafCheckpointId: Felt): Promise<QHashOut> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getCheckpointTreeLeafHashF(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProof(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProofF(checkpointId: Felt, leafCheckpointId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())!
            .getCheckpointTreeMerkleProofF(checkpointId, leafCheckpointId);
    }
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<PsyCheckpointGlobalStateRoots> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointGlobalStateRoots(checkpointId);
    }
    getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<PsyCheckpointSyncInfoCompact> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getCheckpointSyncInfoCompact(checkpointId);
    }
    latestCheckpoint(): Promise<number> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.latestCheckpoint();
    }
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<PsyUserLeaf> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserLeafData(checkpointId, userId);
    }
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserTreeMerkleProof(checkpointId, userId);
    }
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>> {
        return this.rpcs.get(this.getCurrentCoordinatorId())!.getUserTreeMerkleProofF(checkpointId, userId);
    }
}
