import { MerkleProofCore, QHashOut, Felt } from "../core";
import {
    CheckpointSyncInfo,
    ContractCodeDefinition,
    LatestCheckpointResponse,
    QBCDeployContract,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDCheckpointSyncInfoCompact,
    QEDContractLeaf,
    QEDL2BlockState,
    QEDUserLeaf,
    ZKPublicKeyInfo,
} from "../types";

/**
 * Coordinator Edge RPC Command namespace
 */
export enum CoordinatorEdgeRPCCommand {
    RegisterUser = "qed_register_user",
    GetUserId = "qed_get_user_id",
    DeployContract = "qed_deploy_contract",
    GetLatestCheckpoint = "qed_get_latest_checkpoint",
    BuildBlock = "qed_build_block",
    GetCheckpointSyncInfo = "qed_get_checkpoint_sync_info",
    GetContractLeafData = "qed_get_contract_leaf_data",
    GetContractLeafDataF = "qed_get_contract_leaf_data_f",
    GetCheckpointLeafData = "qed_get_checkpoint_leaf_data",
    GetCheckpointLeafDataF = "qed_get_checkpoint_leaf_data_f",
    GetContractCodeDefinition = "qed_get_contract_code_definition",
    GetContractCodeDefinitionF = "qed_get_contract_code_definition_f",
    GetLatestL2BlockState = "qed_get_latest_l2_block_state",
    GetL2BlockState = "qed_get_l2_block_state",
    GetL2BlockStateF = "qed_get_l2_block_state_f",
    GetUserRegistrationTreeRoot = "qed_get_user_registration_tree_root",
    GetUserRegistrationTreeRootF = "qed_get_user_registration_tree_root_f",
    GetUserRegistrationTreeLeafHash = "qed_get_user_registration_tree_leaf_hash",
    GetUserRegistrationTreeLeafHashF = "qed_get_user_registration_tree_leaf_hash_f",
    GetUserRegistrationTreeMerkleProof = "qed_get_user_registration_tree_merkle_proof",
    GetUserRegistrationTreeMerkleProofF = "qed_get_user_registration_tree_merkle_proof_f",
    GetUserTreeRoot = "qed_get_user_tree_root",
    GetUserTreeRootF = "qed_get_user_tree_root_f",
    GetUserSubTreeMerkleProof = "qed_get_user_sub_tree_merkle_proof",
    GetUserTopTreeMerkleProof = "qed_get_user_top_tree_merkle_proof",
    GetUserTopTreeCapRoot = "qed_get_user_top_tree_cap_root",
    GetUserLatestTopTreeCapRoot = "qed_get_user_latest_top_tree_cap_root",
    GetContractFunctionTreeRoot = "qed_get_contract_function_tree_root",
    GetContractFunctionTreeRootF = "qed_get_contract_function_tree_root_f",
    GetContractFunctionTreeLeafHash = "qed_get_contract_function_tree_leaf_hash",
    GetContractFunctionTreeLeafHashF = "qed_get_contract_function_tree_leaf_hash_f",
    GetContractFunctionTreeMerkleProof = "qed_get_contract_function_tree_merkle_proof",
    GetContractFunctionTreeMerkleProofF = "qed_get_contract_function_tree_merkle_proof_f",
    GetContractTreeRoot = "qed_get_contract_tree_root",
    GetContractTreeRootF = "qed_get_contract_tree_root_f",
    GetContractTreeLeafHash = "qed_get_contract_tree_leaf_hash",
    GetContractTreeLeafHashF = "qed_get_contract_tree_leaf_hash_f",
    GetContractTreeMerkleProof = "qed_get_contract_tree_merkle_proof",
    GetContractTreeMerkleProofF = "qed_get_contract_tree_merkle_proof_f",
    GetDepositTreeRoot = "qed_get_deposit_tree_root",
    GetDepositTreeRootF = "qed_get_deposit_tree_root_f",
    GetDepositTreeLeafHash = "qed_get_deposit_tree_leaf_hash",
    GetDepositTreeLeafHashF = "qed_get_deposit_tree_leaf_hash_f",
    GetDepositTreeMerkleProof = "qed_get_deposit_tree_merkle_proof",
    GetDepositTreeMerkleProofF = "qed_get_deposit_tree_merkle_proof_f",
    GetWithdrawalTreeRoot = "qed_get_withdrawal_tree_root",
    GetWithdrawalTreeRootF = "qed_get_withdrawal_tree_root_f",
    GetWithdrawalTreeLeafHash = "qed_get_withdrawal_tree_leaf_hash",
    GetWithdrawalTreeLeafHashF = "qed_get_withdrawal_tree_leaf_hash_f",
    GetWithdrawalTreeMerkleProof = "qed_get_withdrawal_tree_merkle_proof",
    GetWithdrawalTreeMerkleProofF = "qed_get_withdrawal_tree_merkle_proof_f",
    GetLatestCheckpointTreeRoot = "qed_get_latest_checkpoint_tree_root",
    GetCheckpointTreeRoot = "qed_get_checkpoint_tree_root",
    GetCheckpointTreeRootF = "qed_get_checkpoint_tree_root_f",
    GetCheckpointTreeLeafHash = "qed_get_checkpoint_tree_leaf_hash",
    GetCheckpointTreeLeafHashF = "qed_get_checkpoint_tree_leaf_hash_f",
    GetCheckpointTreeMerkleProof = "qed_get_checkpoint_tree_merkle_proof",
    GetCheckpointTreeMerkleProofF = "qed_get_checkpoint_tree_merkle_proof_f",
    GetCheckpointGlobalStateRoots = "qed_get_checkpoint_global_state_roots",
    GetCheckpointSyncInfoCompact = "qed_get_checkpoint_sync_info_compact",
    LatestCheckpoint = "qed_latest_checkpoint",
    GetUserLeafData = "qed_get_user_leaf_data",
    GetUserTreeMerkleProof = "qed_get_user_tree_merkle_proof",
    GetUserTreeMerkleProofF = "qed_get_user_tree_merkle_proof_f",
}

/**
 * Interface for the Coordinator Edge RPC Provider
 */
export interface ICoordinatorEdgeRpcProvider {
    registerUser(pubKey: ZKPublicKeyInfo): Promise<string>;
    getUserId(publicKey: QHashOut): Promise<number>;
    deployContract(contract: QBCDeployContract): Promise<string>;
    getLatestCheckpoint(): Promise<LatestCheckpointResponse>;
    buildBlock(): Promise<string>;
    getCheckpointSyncInfo(checkpointId: Felt): Promise<CheckpointSyncInfo>;
    getContractLeafData(contractId: Felt): Promise<QEDContractLeaf>;
    getContractLeafDataF(contractId: Felt): Promise<QEDContractLeaf>;
    getCheckpointLeafData(checkpointId: Felt): Promise<QEDCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: Felt): Promise<QEDCheckpointLeaf>;
    getContractCodeDefinition(contractId: Felt): Promise<ContractCodeDefinition>;
    getContractCodeDefinitionF(contractId: Felt): Promise<ContractCodeDefinition>;
    getLatestL2BlockState(): Promise<QEDL2BlockState>;
    getL2BlockState(checkpointId: Felt): Promise<QEDL2BlockState>;
    getL2BlockStateF(checkpointId: Felt): Promise<QEDL2BlockState>;
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
    getCheckpointGlobalStateRoots(checkpointId: Felt): Promise<QEDCheckpointGlobalStateRoots>;
    getCheckpointSyncInfoCompact(checkpointId: Felt): Promise<QEDCheckpointSyncInfoCompact>;
    latestCheckpoint(): Promise<number>;
    getUserLeafData(checkpointId: Felt, userId: Felt): Promise<QEDUserLeaf>;
    getUserTreeMerkleProof(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: Felt, userId: Felt): Promise<MerkleProofCore<QHashOut>>;
}
