'use strict';

var types = require('./types.cjs');
var provider = require('../provider/provider.cjs');
require('../utils/felt.cjs');
require('../utils/json.cjs');
require('../utils/random.cjs');

/**
 * Enhanced implementation of the Coordinator Edge RPC Provider with caching, retry logic, and multi-provider support
 */
class CoordinatorEdgeRpcProvider extends provider.Provider {
    constructor(urlOrUrls, configOrHttpClient, httpClient) {
        super(urlOrUrls, configOrHttpClient, httpClient);
        // Read-only methods that can be cached
        this.readOnlyMethods = new Set([
            types.CoordinatorEdgeRPCCommand.GetUserId,
            types.CoordinatorEdgeRPCCommand.GetLatestCheckpointId,
            types.CoordinatorEdgeRPCCommand.GetCheckpointSyncInfo,
            types.CoordinatorEdgeRPCCommand.GetContractLeafData,
            types.CoordinatorEdgeRPCCommand.GetContractLeafDataF,
            types.CoordinatorEdgeRPCCommand.GetCheckpointLeafData,
            types.CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF,
            types.CoordinatorEdgeRPCCommand.GetContractCodeDefinition,
            types.CoordinatorEdgeRPCCommand.GetContractCodeDefinitionF,
            types.CoordinatorEdgeRPCCommand.GetLatestBlockState,
            types.CoordinatorEdgeRPCCommand.GetBlockState,
            types.CoordinatorEdgeRPCCommand.GetBlockStateF,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetUserTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetUserTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetUserSubTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetUserTopTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetUserTopTreeCapRoot,
            types.CoordinatorEdgeRPCCommand.GetUserLatestTopTreeCapRoot,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetContractTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetContractTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetContractTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetContractTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetContractTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetContractTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetLatestCheckpointTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeRoot,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeRootF,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHash,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHashF,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProofF,
            types.CoordinatorEdgeRPCCommand.GetCheckpointGlobalStateRoots,
            types.CoordinatorEdgeRPCCommand.GetCheckpointSyncInfoCompact,
            types.CoordinatorEdgeRPCCommand.LatestCheckpoint,
            types.CoordinatorEdgeRPCCommand.GetUserLeafData,
            types.CoordinatorEdgeRPCCommand.GetUserTreeMerkleProof,
            types.CoordinatorEdgeRPCCommand.GetUserTreeMerkleProofF,
        ]);
    }
    /**
     * Get read-only methods for caching
     */
    getReadOnlyMethods() {
        return this.readOnlyMethods;
    }
    /**
     * Get health check method
     */
    getHealthCheckMethod() {
        return types.CoordinatorEdgeRPCCommand.GetLatestCheckpointId;
    }
    /**
     * Register a user with their ZK public key
     * @param pubKey The ZK public key info
     * @returns A confirmation message
     */
    async registerUser(pubKey) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.RegisterUser, [pubKey]);
    }
    /**
     * Get a user ID from a QHash
     * @param qhash The QHash of the user's public key
     * @returns The user ID
     */
    async getUserId(publicKey) {
        const result = await this.rpc(types.CoordinatorEdgeRPCCommand.GetUserId, { public_key: publicKey, start_user_id: 0, count: 64 });
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
    async deployContract(contract) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.DeployContract, [contract]);
    }
    /**
     * Get the latest checkpoint Id information
     * @returns The latest checkpoint response
     */
    async getLatestCheckpointId() {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetLatestCheckpointId, []);
    }
    /**
     * Build a new block
     * @returns A confirmation message
     */
    async buildBlock() {
        return this.rpc(types.CoordinatorEdgeRPCCommand.BuildBlock, []);
    }
    /**
     * Get checkpoint sync information
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint sync information
     */
    async getCheckpointSyncInfo(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointSyncInfo, [checkpointId]);
    }
    /**
     * Get contract leaf data
     * @param contractId The contract ID
     * @returns Contract leaf data
     */
    async getContractLeafData(contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractLeafData, [contractId]);
    }
    /**
     * Get contract leaf data with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract leaf data
     */
    async getContractLeafDataF(contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractLeafDataF, [contractId]);
    }
    /**
     * Get checkpoint leaf data
     * @param checkpointId The checkpoint ID
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafData(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }
    /**
     * Get checkpoint leaf data with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns Checkpoint leaf data
     */
    async getCheckpointLeafDataF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }
    /**
     * Get contract code definition
     * @param contractId The contract ID
     * @returns Contract code definition
     */
    async getContractCodeDefinition(contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractCodeDefinition, [contractId]);
    }
    /**
     * Get contract code definition with field element
     * @param contractId The contract ID as a bigint
     * @returns Contract code definition
     */
    async getContractCodeDefinitionF(contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractCodeDefinitionF, [contractId]);
    }
    /**
     * Get latest block state
     * @returns block state
     */
    async getLatestBlockState() {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetLatestBlockState, []);
    }
    /**
     * Get block state for a specific checkpoint
     * @param checkpointId The checkpoint ID
     * @returns block state
     */
    async getBlockState(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetBlockState, [checkpointId]);
    }
    /**
     * Get block state with field element
     * @param checkpointId The checkpoint ID as a bigint
     * @returns block state
     */
    async getBlockStateF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetBlockStateF, [checkpointId]);
    }
    // User Registration Tree methods
    async getUserRegistrationTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }
    async getUserRegistrationTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeRootF, [checkpointId]);
    }
    async getUserRegistrationTreeLeafHash(checkpointId, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHash, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }
    async getUserRegistrationTreeLeafHashF(checkpointId, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeLeafHashF, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }
    async getUserRegistrationTreeMerkleProof(checkpointId, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProof, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }
    async getUserRegistrationTreeMerkleProofF(checkpointId, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserRegistrationTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            leaf_index: leafIndex,
        });
    }
    // User Tree methods
    async getUserTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }
    async getUserTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }
    async getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserSubTreeMerkleProof, {
            checkpoint_id: checkpointId,
            root_level: rootLevel,
            leaf_level: leafLevel,
            leaf_index: leafIndex,
        });
    }
    async getUserTopTreeMerkleProof(checkpointId, leafLevel, leafIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTopTreeMerkleProof, [
            checkpointId,
            leafLevel,
            leafIndex,
        ]);
    }
    async getUserTopTreeCapRoot(checkpointId, capLevel, capIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTopTreeCapRoot, [checkpointId, capLevel, capIndex]);
    }
    async getUserLatestTopTreeCapRoot(capLevel, capIndex) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserLatestTopTreeCapRoot, [capLevel, capIndex]);
    }
    // Contract Function Tree methods
    async getContractFunctionTreeRoot(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeRoot, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    async getContractFunctionTreeRootF(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeRootF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    async getContractFunctionTreeLeafHash(checkpointId, contractId, functionId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHash, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }
    async getContractFunctionTreeLeafHashF(checkpointId, contractId, functionId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeLeafHashF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }
    async getContractFunctionTreeMerkleProof(checkpointId, contractId, functionId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProof, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }
    async getContractFunctionTreeMerkleProofF(checkpointId, contractId, functionId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractFunctionTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
            function_id: functionId,
        });
    }
    // Contract Tree methods
    async getContractTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeRoot, [checkpointId]);
    }
    async getContractTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeRootF, [checkpointId]);
    }
    async getContractTreeLeafHash(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeLeafHash, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    async getContractTreeLeafHashF(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeLeafHashF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    async getContractTreeMerkleProof(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeMerkleProof, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    async getContractTreeMerkleProofF(checkpointId, contractId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetContractTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            contract_id: contractId,
        });
    }
    // Deposit Tree methods
    async getDepositTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeRoot, [checkpointId]);
    }
    async getDepositTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeRootF, [checkpointId]);
    }
    async getDepositTreeLeafHash(checkpointId, depositId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeLeafHash, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }
    async getDepositTreeLeafHashF(checkpointId, depositId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeLeafHashF, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }
    async getDepositTreeMerkleProof(checkpointId, depositId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProof, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }
    async getDepositTreeMerkleProofF(checkpointId, depositId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetDepositTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            deposit_id: depositId,
        });
    }
    // Withdrawal Tree methods
    async getWithdrawalTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeRoot, [checkpointId]);
    }
    async getWithdrawalTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeRootF, [checkpointId]);
    }
    async getWithdrawalTreeLeafHash(checkpointId, withdrawalId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHash, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }
    async getWithdrawalTreeLeafHashF(checkpointId, withdrawalId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeLeafHashF, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }
    async getWithdrawalTreeMerkleProof(checkpointId, withdrawalId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProof, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }
    async getWithdrawalTreeMerkleProofF(checkpointId, withdrawalId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetWithdrawalTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            withdrawal_id: withdrawalId,
        });
    }
    // Checkpoint Tree methods
    async getLatestCheckpointTreeRoot() {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }
    async getCheckpointTreeRoot(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }
    async getCheckpointTreeRootF(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }
    async getCheckpointTreeLeafHash(checkpointId, leafCheckpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHash, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }
    async getCheckpointTreeLeafHashF(checkpointId, leafCheckpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeLeafHashF, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }
    async getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProof, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }
    async getCheckpointTreeMerkleProofF(checkpointId, leafCheckpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            leaf_checkpoint_id: leafCheckpointId,
        });
    }
    // Global state and checkpoint info methods
    async getCheckpointGlobalStateRoots(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointGlobalStateRoots, [
            checkpointId,
        ]);
    }
    async getCheckpointSyncInfoCompact(checkpointId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetCheckpointSyncInfoCompact, checkpointId);
    }
    async latestCheckpoint() {
        return this.rpc(types.CoordinatorEdgeRPCCommand.LatestCheckpoint, []);
    }
    // User data methods
    async getUserLeafData(checkpointId, userId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserLeafData, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }
    async getUserTreeMerkleProof(checkpointId, userId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTreeMerkleProof, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }
    async getUserTreeMerkleProofF(checkpointId, userId) {
        return this.rpc(types.CoordinatorEdgeRPCCommand.GetUserTreeMerkleProofF, {
            checkpoint_id: checkpointId,
            user_id: userId,
        });
    }
}
class MultiCoordinatorRpcProvider {
    constructor(coordinatorRpcConfigs) {
        this.rpcs = new Map();
        for (const coordinatorRpcConfig of coordinatorRpcConfigs) {
            this.rpcs.set(coordinatorRpcConfig.id, new CoordinatorEdgeRpcProvider(coordinatorRpcConfig.rpc_url));
        }
    }
    getCurrentCoordinatorId() {
        return 0;
    }
    registerUser(pubKey) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).registerUser(pubKey);
    }
    getUserId(publicKey) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserId(publicKey);
    }
    deployContract(contract) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).deployContract(contract);
    }
    getLatestCheckpointId() {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getLatestCheckpointId();
    }
    buildBlock() {
        return this.rpcs.get(this.getCurrentCoordinatorId()).buildBlock();
    }
    getCheckpointSyncInfo(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointSyncInfo(checkpointId);
    }
    getContractLeafData(contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractLeafData(contractId);
    }
    getContractLeafDataF(contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractLeafDataF(contractId);
    }
    getCheckpointLeafData(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointLeafData(checkpointId);
    }
    getCheckpointLeafDataF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointLeafDataF(checkpointId);
    }
    getContractCodeDefinition(contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractCodeDefinition(contractId);
    }
    getContractCodeDefinitionF(contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractCodeDefinitionF(contractId);
    }
    getLatestBlockState() {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getLatestBlockState();
    }
    getBlockState(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getBlockState(checkpointId);
    }
    getBlockStateF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getBlockStateF(checkpointId);
    }
    getUserRegistrationTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserRegistrationTreeRoot(checkpointId);
    }
    getUserRegistrationTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserRegistrationTreeRootF(checkpointId);
    }
    getUserRegistrationTreeLeafHash(checkpointId, leafIndex) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserRegistrationTreeLeafHash(checkpointId, leafIndex);
    }
    getUserRegistrationTreeLeafHashF(checkpointId, leafIndex) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserRegistrationTreeLeafHashF(checkpointId, leafIndex);
    }
    getUserRegistrationTreeMerkleProof(checkpointId, leafIndex) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getUserRegistrationTreeMerkleProof(checkpointId, leafIndex);
    }
    getUserRegistrationTreeMerkleProofF(checkpointId, leafIndex) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getUserRegistrationTreeMerkleProofF(checkpointId, leafIndex);
    }
    getUserTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserTreeRoot(checkpointId);
    }
    getUserTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserTreeRootF(checkpointId);
    }
    getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex);
    }
    getUserTopTreeMerkleProof(checkpointId, leafLevel, leafIndex) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getUserTopTreeMerkleProof(checkpointId, leafLevel, leafIndex);
    }
    getUserTopTreeCapRoot(checkpointId, capLevel, capIndex) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserTopTreeCapRoot(checkpointId, capLevel, capIndex);
    }
    getUserLatestTopTreeCapRoot(capLevel, capIndex) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserLatestTopTreeCapRoot(capLevel, capIndex);
    }
    getContractFunctionTreeRoot(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractFunctionTreeRoot(checkpointId, contractId);
    }
    getContractFunctionTreeRootF(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractFunctionTreeRootF(checkpointId, contractId);
    }
    getContractFunctionTreeLeafHash(checkpointId, contractId, functionId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getContractFunctionTreeLeafHash(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeLeafHashF(checkpointId, contractId, functionId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getContractFunctionTreeLeafHashF(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeMerkleProof(checkpointId, contractId, functionId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getContractFunctionTreeMerkleProof(checkpointId, contractId, functionId);
    }
    getContractFunctionTreeMerkleProofF(checkpointId, contractId, functionId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getContractFunctionTreeMerkleProofF(checkpointId, contractId, functionId);
    }
    getContractTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeRoot(checkpointId);
    }
    getContractTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeRootF(checkpointId);
    }
    getContractTreeLeafHash(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeLeafHash(checkpointId, contractId);
    }
    getContractTreeLeafHashF(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeLeafHashF(checkpointId, contractId);
    }
    getContractTreeMerkleProof(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeMerkleProof(checkpointId, contractId);
    }
    getContractTreeMerkleProofF(checkpointId, contractId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getContractTreeMerkleProofF(checkpointId, contractId);
    }
    getDepositTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeRoot(checkpointId);
    }
    getDepositTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeRootF(checkpointId);
    }
    getDepositTreeLeafHash(checkpointId, depositId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeLeafHash(checkpointId, depositId);
    }
    getDepositTreeLeafHashF(checkpointId, depositId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeLeafHashF(checkpointId, depositId);
    }
    getDepositTreeMerkleProof(checkpointId, depositId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeMerkleProof(checkpointId, depositId);
    }
    getDepositTreeMerkleProofF(checkpointId, depositId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getDepositTreeMerkleProofF(checkpointId, depositId);
    }
    getWithdrawalTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeRoot(checkpointId);
    }
    getWithdrawalTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeRootF(checkpointId);
    }
    getWithdrawalTreeLeafHash(checkpointId, withdrawalId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeLeafHash(checkpointId, withdrawalId);
    }
    getWithdrawalTreeLeafHashF(checkpointId, withdrawalId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeLeafHashF(checkpointId, withdrawalId);
    }
    getWithdrawalTreeMerkleProof(checkpointId, withdrawalId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeMerkleProof(checkpointId, withdrawalId);
    }
    getWithdrawalTreeMerkleProofF(checkpointId, withdrawalId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getWithdrawalTreeMerkleProofF(checkpointId, withdrawalId);
    }
    getLatestCheckpointTreeRoot() {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getLatestCheckpointTreeRoot();
    }
    getCheckpointTreeRoot(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointTreeRoot(checkpointId);
    }
    getCheckpointTreeRootF(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointTreeRootF(checkpointId);
    }
    getCheckpointTreeLeafHash(checkpointId, leafCheckpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointTreeLeafHash(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeLeafHashF(checkpointId, leafCheckpointId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getCheckpointTreeLeafHashF(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId);
    }
    getCheckpointTreeMerkleProofF(checkpointId, leafCheckpointId) {
        return this.rpcs
            .get(this.getCurrentCoordinatorId())
            .getCheckpointTreeMerkleProofF(checkpointId, leafCheckpointId);
    }
    getCheckpointGlobalStateRoots(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointGlobalStateRoots(checkpointId);
    }
    getCheckpointSyncInfoCompact(checkpointId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getCheckpointSyncInfoCompact(checkpointId);
    }
    latestCheckpoint() {
        return this.rpcs.get(this.getCurrentCoordinatorId()).latestCheckpoint();
    }
    getUserLeafData(checkpointId, userId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserLeafData(checkpointId, userId);
    }
    getUserTreeMerkleProof(checkpointId, userId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserTreeMerkleProof(checkpointId, userId);
    }
    getUserTreeMerkleProofF(checkpointId, userId) {
        return this.rpcs.get(this.getCurrentCoordinatorId()).getUserTreeMerkleProofF(checkpointId, userId);
    }
}

exports.CoordinatorEdgeRpcProvider = CoordinatorEdgeRpcProvider;
exports.MultiCoordinatorRpcProvider = MultiCoordinatorRpcProvider;
