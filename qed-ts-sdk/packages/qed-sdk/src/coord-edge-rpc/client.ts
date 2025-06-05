import { CoordinatorEdgeRPCCommand, ICoordinatorEdgeRpcProvider } from "./types";

import { QHashOut, MerkleProofCore, Felt } from "../core";
import { IHTTPClient } from "../http";
import { Provider, ClientConfig } from "../provider";
import {
    CheckpointSyncInfo,
    ContractCodeDefinition,
    LatestCheckpointResponse,
    QBCDeployContract,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDCheckpointSyncInfoCompact,
    QEDContractLeaf,
    QEDUserLeaf,
    QEDL2BlockState,
    ZKPublicKeyInfo,
} from "../types";

/**
 * Enhanced implementation of the Coordinator Edge RPC Provider with caching, retry logic, and multi-provider support
 */
export class CoordinatorEdgeRpcProvider extends Provider implements ICoordinatorEdgeRpcProvider {
    // Read-only methods that can be cached
    private readonly readOnlyMethods = new Set<string>([
        CoordinatorEdgeRPCCommand.GetUserId,
        CoordinatorEdgeRPCCommand.GetLatestCheckpoint,
        CoordinatorEdgeRPCCommand.GetCheckpointSyncInfo,
        CoordinatorEdgeRPCCommand.GetContractLeafData,
        CoordinatorEdgeRPCCommand.GetContractLeafDataF,
        CoordinatorEdgeRPCCommand.GetCheckpointLeafData,
        CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF,
        CoordinatorEdgeRPCCommand.GetContractCodeDefinition,
        CoordinatorEdgeRPCCommand.GetContractCodeDefinitionF,
        CoordinatorEdgeRPCCommand.GetLatestL2BlockState,
        CoordinatorEdgeRPCCommand.GetL2BlockState,
        CoordinatorEdgeRPCCommand.GetL2BlockStateF,
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
        return CoordinatorEdgeRPCCommand.GetLatestCheckpoint;
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
    async getUserId(qhash: QHashOut): Promise<number> {
        return this.rpc<number>(CoordinatorEdgeRPCCommand.GetUserId, qhash);
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
     * Get the latest checkpoint information
     * @returns The latest checkpoint response
     */
    async getLatestCheckpoint(): Promise<LatestCheckpointResponse> {
        return this.rpc<LatestCheckpointResponse>(CoordinatorEdgeRPCCommand.GetLatestCheckpoint, []);
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
    async getContractLeafData(contractId: Felt): Promise<QEDContractLeaf> {
        return this.rpc<QEDContractLeaf>(CoordinatorEdgeRPCCommand.GetContractLeafData, [contractId]);
    }

    /**
     * Get contract leaf data with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract leaf data
     */
    async getContractLeafDataF(contractId: Felt): Promise<QEDContractLeaf> {
        return this.rpc<QEDContractLeaf>(CoordinatorEdgeRPCCommand.GetContractLeafDataF, [contractId]);
    }

    /**
     * Get checkpoint leaf data
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafData(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc<QEDCheckpointLeaf>(CoordinatorEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    /**
     * Get checkpoint leaf data with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafDataF(checkpointId: Felt): Promise<QEDCheckpointLeaf> {
        return this.rpc<QEDCheckpointLeaf>(CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
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
     * Get latest L2 block state
     * @returns L2 block state
     */
    async getLatestL2BlockState(): Promise<QEDL2BlockState> {
        return this.rpc<QEDL2BlockState>(CoordinatorEdgeRPCCommand.GetLatestL2BlockState, []);
    }

    /**
     * Get L2 block state for a specific checkpoint
     * @param checkpointId The checkpoint ID
     * @returns L2 block state
     */
    async getL2BlockState(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc<QEDL2BlockState>(CoordinatorEdgeRPCCommand.GetL2BlockState, [checkpointId]);
    }

    /**
     * Get L2 block state with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns L2 block state
     */
    async getL2BlockStateF(checkpointId: Felt): Promise<QEDL2BlockState> {
        return this.rpc<QEDL2BlockState>(CoordinatorEdgeRPCCommand.GetL2BlockStateF, [checkpointId]);
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
    async getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<QEDCheckpointGlobalStateRoots> {
        return this.rpc<QEDCheckpointGlobalStateRoots>(CoordinatorEdgeRPCCommand.GetCheckpointGlobalStateRoots, [
            checkpointId,
        ]);
    }

    async getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<QEDCheckpointSyncInfoCompact> {
        return this.rpc<QEDCheckpointSyncInfoCompact>(
            CoordinatorEdgeRPCCommand.GetCheckpointSyncInfoCompact,
            checkpointId
        );
    }

    async latestCheckpoint(): Promise<number> {
        return this.rpc<number>(CoordinatorEdgeRPCCommand.LatestCheckpoint, []);
    }

    // User data methods
    async getUserLeafData(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf> {
        return this.rpc<QEDUserLeaf>(CoordinatorEdgeRPCCommand.GetUserLeafData, {
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
