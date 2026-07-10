import { MerkleProofCore, QHashOut, Felt } from "../core";
import {
    CheckpointSyncInfo,
    ContractCodeDefinition,
    QBCDeployContract,
    PsyCheckpointGlobalStateRoots,
    PsyCheckpointLeaf,
    PsyCheckpointSyncInfoCompact,
    PsyContractLeaf,
    PsyBlockState,
    PsyUserLeaf,
    ZKPublicKeyInfo,
} from "../types";

/**
 * Coordinator Edge RPC Command namespace
 */
export enum CoordinatorEdgeRPCCommand {
    RegisterUser = "psy_register_user",
    GetUserId = "psy_get_user_ids_for_public_key",
    DeployContract = "psy_deploy_contract",
    GetLatestCheckpointId = "psy_get_latest_checkpoint_id",
    BuildBlock = "psy_build_block",
    GetCheckpointSyncInfo = "psy_get_checkpoint_sync_info",
    GetContractLeafData = "psy_get_contract_leaf_data",
    GetContractLeafDataF = "psy_get_contract_leaf_data_f",
    GetCheckpointLeafData = "psy_get_checkpoint_leaf_data",
    GetCheckpointLeafDataF = "psy_get_checkpoint_leaf_data_f",
    GetContractCodeDefinition = "psy_get_contract_code_definition",
    GetContractCodeDefinitionF = "psy_get_contract_code_definition_f",
    GetLatestBlockState = "psy_get_latest_l2_block_state",
    GetBlockState = "psy_get_l2_block_state",
    GetBlockStateF = "psy_get_l2_block_state_f",
    GetUserRegistrationTreeRoot = "psy_get_user_registration_tree_root",
    GetUserRegistrationTreeRootF = "psy_get_user_registration_tree_root_f",
    GetUserRegistrationTreeLeafHash = "psy_get_user_registration_tree_leaf_hash",
    GetUserRegistrationTreeLeafHashF = "psy_get_user_registration_tree_leaf_hash_f",
    GetUserRegistrationTreeMerkleProof = "psy_get_user_registration_tree_merkle_proof",
    GetUserRegistrationTreeMerkleProofF = "psy_get_user_registration_tree_merkle_proof_f",
    GetUserTreeRoot = "psy_get_user_tree_root",
    GetUserTreeRootF = "psy_get_user_tree_root_f",
    GetUserSubTreeMerkleProof = "psy_get_user_sub_tree_merkle_proof",
    GetUserTopTreeMerkleProof = "psy_get_user_top_tree_merkle_proof",
    GetUserTopTreeCapRoot = "psy_get_user_top_tree_cap_root",
    GetUserLatestTopTreeCapRoot = "psy_get_user_latest_top_tree_cap_root",
    GetContractFunctionTreeRoot = "psy_get_contract_function_tree_root",
    GetContractFunctionTreeRootF = "psy_get_contract_function_tree_root_f",
    GetContractFunctionTreeLeafHash = "psy_get_contract_function_tree_leaf_hash",
    GetContractFunctionTreeLeafHashF = "psy_get_contract_function_tree_leaf_hash_f",
    GetContractFunctionTreeMerkleProof = "psy_get_contract_function_tree_merkle_proof",
    GetContractFunctionTreeMerkleProofF = "psy_get_contract_function_tree_merkle_proof_f",
    GetContractTreeRoot = "psy_get_contract_tree_root",
    GetContractTreeRootF = "psy_get_contract_tree_root_f",
    GetContractTreeLeafHash = "psy_get_contract_tree_leaf_hash",
    GetContractTreeLeafHashF = "psy_get_contract_tree_leaf_hash_f",
    GetContractTreeMerkleProof = "psy_get_contract_tree_merkle_proof",
    GetContractTreeMerkleProofF = "psy_get_contract_tree_merkle_proof_f",
    GetDepositTreeRoot = "psy_get_deposit_tree_root",
    GetDepositTreeRootF = "psy_get_deposit_tree_root_f",
    GetDepositTreeLeafHash = "psy_get_deposit_tree_leaf_hash",
    GetDepositTreeLeafHashF = "psy_get_deposit_tree_leaf_hash_f",
    GetDepositTreeMerkleProof = "psy_get_deposit_tree_merkle_proof",
    GetDepositTreeMerkleProofF = "psy_get_deposit_tree_merkle_proof_f",
    GetWithdrawalTreeRoot = "psy_get_withdrawal_tree_root",
    GetWithdrawalTreeRootF = "psy_get_withdrawal_tree_root_f",
    GetWithdrawalTreeLeafHash = "psy_get_withdrawal_tree_leaf_hash",
    GetWithdrawalTreeLeafHashF = "psy_get_withdrawal_tree_leaf_hash_f",
    GetWithdrawalTreeMerkleProof = "psy_get_withdrawal_tree_merkle_proof",
    GetWithdrawalTreeMerkleProofF = "psy_get_withdrawal_tree_merkle_proof_f",
    GetLatestCheckpointTreeRoot = "psy_get_latest_checkpoint_tree_root",
    GetCheckpointTreeRoot = "psy_get_checkpoint_tree_root",
    GetCheckpointTreeRootF = "psy_get_checkpoint_tree_root_f",
    GetCheckpointTreeLeafHash = "psy_get_checkpoint_tree_leaf_hash",
    GetCheckpointTreeLeafHashF = "psy_get_checkpoint_tree_leaf_hash_f",
    GetCheckpointTreeMerkleProof = "psy_get_checkpoint_tree_merkle_proof",
    GetCheckpointTreeMerkleProofF = "psy_get_checkpoint_tree_merkle_proof_f",
    GetCheckpointGlobalStateRoots = "psy_get_checkpoint_global_state_roots",
    GetCheckpointSyncInfoCompact = "psy_get_checkpoint_sync_info_compact",
    LatestCheckpoint = "psy_latest_checkpoint",
    GetUserLeafData = "psy_get_user_leaf_data",
    GetUserTreeMerkleProof = "psy_get_user_tree_merkle_proof",
    GetUserTreeMerkleProofF = "psy_get_user_tree_merkle_proof_f",
}

/**
 * Interface for the Coordinator Edge RPC Provider
 */
export interface ICoordinatorEdgeRpcProvider {
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string>;
    getUserId(publicKey: QHashOut): Promise<number>;
    deployContract(contract: QBCDeployContract, signal?: AbortSignal): Promise<string>;
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
    getUserSubTreeMerkleProof(
        checkpointId: Felt,
        rootLevel: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeMerkleProof(
        checkpointId: Felt,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeCapRoot(checkpointId: Felt, capLevel: number, capIndex: number): Promise<QHashOut>;
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut>;
    getContractFunctionTreeRoot(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractFunctionTreeRootF(checkpointId: Felt, contractId: Felt): Promise<QHashOut>;
    getContractFunctionTreeLeafHash(checkpointId: Felt, contractId: Felt, functionId: number): Promise<QHashOut>;
    getContractFunctionTreeLeafHashF(checkpointId: Felt, contractId: Felt, functionId: bigint): Promise<QHashOut>;
    getContractFunctionTreeMerkleProof(
        checkpointId: Felt,
        contractId: Felt,
        functionId: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getContractFunctionTreeMerkleProofF(
        checkpointId: Felt,
        contractId: Felt,
        functionId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;
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
