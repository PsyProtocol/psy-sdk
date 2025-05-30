import { MerkleProofCore, QHashOut } from "../core";
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
    getUserId(qhash: QHashOut): Promise<number>;
    deployContract(contract: QBCDeployContract): Promise<string>;
    getLatestCheckpoint(): Promise<LatestCheckpointResponse>;
    buildBlock(): Promise<string>;
    getCheckpointSyncInfo(checkpointId: number): Promise<CheckpointSyncInfo>;
    getContractLeafData(contractId: number): Promise<QEDContractLeaf>;
    getContractLeafDataF(contractId: bigint | number): Promise<QEDContractLeaf>;
    getCheckpointLeafData(checkpointId: number): Promise<QEDCheckpointLeaf>;
    getCheckpointLeafDataF(checkpointId: bigint): Promise<QEDCheckpointLeaf>;
    getContractCodeDefinition(contractId: number): Promise<ContractCodeDefinition>;
    getContractCodeDefinitionF(contractId: bigint): Promise<ContractCodeDefinition>;
    getLatestL2BlockState(): Promise<QEDL2BlockState>;
    getL2BlockState(checkpointId: number): Promise<QEDL2BlockState>;
    getL2BlockStateF(checkpointId: bigint): Promise<QEDL2BlockState>;
    getUserRegistrationTreeRoot(checkpointId: number): Promise<QHashOut>;
    getUserRegistrationTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getUserRegistrationTreeLeafHash(checkpointId: number, leafIndex: number): Promise<QHashOut>;
    getUserRegistrationTreeLeafHashF(checkpointId: bigint, leafIndex: bigint): Promise<QHashOut>;
    getUserRegistrationTreeMerkleProof(checkpointId: number, leafIndex: number): Promise<MerkleProofCore<QHashOut>>;
    getUserRegistrationTreeMerkleProofF(checkpointId: bigint, leafIndex: bigint): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeRoot(checkpointId: number): Promise<QHashOut>;
    getUserTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getUserSubTreeMerkleProof(
        checkpointId: number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeMerkleProof(
        checkpointId: number,
        leafLevel: number,
        leafIndex: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getUserTopTreeCapRoot(checkpointId: number, capLevel: number, capIndex: number): Promise<QHashOut>;
    getUserLatestTopTreeCapRoot(capLevel: number, capIndex: number): Promise<QHashOut>;
    getContractFunctionTreeRoot(checkpointId: number, contractId: number): Promise<QHashOut>;
    getContractFunctionTreeRootF(checkpointId: bigint, contractId: bigint): Promise<QHashOut>;
    getContractFunctionTreeLeafHash(checkpointId: number, contractId: number, functionId: number): Promise<QHashOut>;
    getContractFunctionTreeLeafHashF(checkpointId: bigint, contractId: bigint, functionId: bigint): Promise<QHashOut>;
    getContractFunctionTreeMerkleProof(
        checkpointId: number,
        contractId: number,
        functionId: number
    ): Promise<MerkleProofCore<QHashOut>>;
    getContractFunctionTreeMerkleProofF(
        checkpointId: bigint,
        contractId: bigint,
        functionId: bigint
    ): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeRoot(checkpointId: number): Promise<QHashOut>;
    getContractTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getContractTreeLeafHash(checkpointId: number, contractId: number): Promise<QHashOut>;
    getContractTreeLeafHashF(checkpointId: bigint, contractId: bigint): Promise<QHashOut>;
    getContractTreeMerkleProof(checkpointId: number, contractId: number): Promise<MerkleProofCore<QHashOut>>;
    getContractTreeMerkleProofF(checkpointId: bigint, contractId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeRoot(checkpointId: number): Promise<QHashOut>;
    getDepositTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getDepositTreeLeafHash(checkpointId: number, depositId: number): Promise<QHashOut>;
    getDepositTreeLeafHashF(checkpointId: bigint, depositId: bigint): Promise<QHashOut>;
    getDepositTreeMerkleProof(checkpointId: number, depositId: number): Promise<MerkleProofCore<QHashOut>>;
    getDepositTreeMerkleProofF(checkpointId: bigint, depositId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeRoot(checkpointId: number): Promise<QHashOut>;
    getWithdrawalTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getWithdrawalTreeLeafHash(checkpointId: number, withdrawalId: number): Promise<QHashOut>;
    getWithdrawalTreeLeafHashF(checkpointId: bigint, withdrawalId: bigint): Promise<QHashOut>;
    getWithdrawalTreeMerkleProof(checkpointId: number, withdrawalId: number): Promise<MerkleProofCore<QHashOut>>;
    getWithdrawalTreeMerkleProofF(checkpointId: bigint, withdrawalId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getLatestCheckpointTreeRoot(): Promise<QHashOut>;
    getCheckpointTreeRoot(checkpointId: number): Promise<QHashOut>;
    getCheckpointTreeRootF(checkpointId: bigint): Promise<QHashOut>;
    getCheckpointTreeLeafHash(checkpointId: number, leafCheckpointId: number): Promise<QHashOut>;
    getCheckpointTreeLeafHashF(checkpointId: bigint, leafCheckpointId: bigint): Promise<QHashOut>;
    getCheckpointTreeMerkleProof(checkpointId: number, leafCheckpointId: number): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointTreeMerkleProofF(checkpointId: bigint, leafCheckpointId: bigint): Promise<MerkleProofCore<QHashOut>>;
    getCheckpointGlobalStateRoots(checkpointId: number): Promise<QEDCheckpointGlobalStateRoots>;
    getCheckpointSyncInfoCompact(checkpointId: number): Promise<QEDCheckpointSyncInfoCompact>;
    latestCheckpoint(): Promise<number>;
    getUserLeafData(checkpointId: number, userId: number): Promise<QEDUserLeaf>;
    getUserTreeMerkleProof(checkpointId: number, userId: number): Promise<MerkleProofCore<QHashOut>>;
    getUserTreeMerkleProofF(checkpointId: bigint, userId: bigint): Promise<MerkleProofCore<QHashOut>>;
}
