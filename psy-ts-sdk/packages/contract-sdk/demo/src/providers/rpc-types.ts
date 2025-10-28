// src/providers/rpc-types.ts

/**
 * RPC methods that should be called on the Coordinator endpoint
 * These handle global operations and cross-realm data
 */
export enum CoordinatorRpcMethods {
    // Checkpoint operations
    LATEST_CHECKPOINT = 'qed_latest_checkpoint',
    GET_CHECKPOINT_LEAF_DATA = 'qed_get_checkpoint_leaf_data',
    GET_CHECKPOINT_GLOBAL_STATE_ROOTS = 'qed_get_checkpoint_global_state_roots',
    GET_CHECKPOINT_TREE_ROOT = 'qed_get_checkpoint_tree_root',
    GET_CHECKPOINT_TREE_LEAF_HASH = 'qed_get_checkpoint_tree_leaf_hash',
    GET_CHECKPOINT_TREE_MERKLE_PROOF = 'qed_get_checkpoint_tree_merkle_proof',

    // Block operations
    BUILD_BLOCK = 'qed_build_block',
    GET_LATEST_L2_BLOCK_STATE = 'qed_get_latest_l2_block_state',
    GET_L2_BLOCK_STATE = 'qed_get_l2_block_state',

    // User operations
    REGISTER_USER = 'qed_register_user',
    GET_USER_LEAF_DATA = 'qed_get_user_leaf_data',
    GET_USER_TREE_ROOT = 'qed_get_user_tree_root',
    GET_USER_SUB_TREE_MERKLE_PROOF = 'qed_get_user_sub_tree_merkle_proof',
    GET_USER_TREE_MERKLE_PROOF = 'qed_get_user_tree_merkle_proof',

    // Contract operations
    DEPLOY_CONTRACT = 'qed_deploy_contract',
    GET_CONTRACT_LEAF_DATA = 'qed_get_contract_leaf_data',
    GET_CONTRACT_CODE_DEFINITION = 'qed_get_contract_code_definition',

    // Transaction submission
    SUBMIT_TRANSACTION = 'qed_submit_transaction', // This might be different in your system
}

/**
 * RPC methods that should be called on the Realm endpoint
 * These handle realm-specific state queries
 */
export enum RealmRpcMethods {
    // User state queries
    CHECK_USER_ID_IN_REALM = 'qed_check_user_id_in_realm',
    GET_USER_TREE_LEAF_HASH = 'qed_get_user_tree_leaf_hash',
    GET_USER_REGISTRATION_TREE_ROOT = 'qed_get_user_registration_tree_root',
    GET_USER_BOTTOM_TREE_MERKLE_PROOF = 'qed_get_user_bottom_tree_merkle_proof',

    // Contract state queries
    GET_USER_CONTRACT_TREE_ROOT = 'qed_get_user_contract_tree_root',
    GET_USER_CONTRACT_STATE_TREE_ROOT = 'qed_get_user_contract_state_tree_root',
    GET_USER_CONTRACT_TREE_MERKLE_PROOF = 'qed_get_user_contract_tree_merkle_proof',
    GET_USER_CONTRACT_STATE_TREE_MERKLE_PROOF = 'qed_get_user_contract_state_tree_merkle_proof',

    // Some methods might be available on both endpoints
    // These are typically replicated for performance
    GET_LATEST_L2_BLOCK_STATE_REALM = 'qed_get_latest_l2_block_state',
    GET_L2_BLOCK_STATE_REALM = 'qed_get_l2_block_state',
    GET_CHECKPOINT_LEAF_DATA_REALM = 'qed_get_checkpoint_leaf_data',
    GET_CHECKPOINT_TREE_ROOT_REALM = 'qed_get_checkpoint_tree_root',
    GET_USER_TREE_ROOT_REALM = 'qed_get_user_tree_root',
}

/**
 * Interface for RPC method categorization
 */
export interface RpcMethodInfo {
    method: string;
    endpoint: 'coordinator' | 'realm' | 'both';
    description: string;
}

/**
 * Complete mapping of all RPC methods
 */
export const RPC_METHOD_MAPPING: RpcMethodInfo[] = [
    // Coordinator-only methods
    { method: 'qed_latest_checkpoint', endpoint: 'coordinator', description: 'Get the latest checkpoint ID' },
    { method: 'qed_register_user', endpoint: 'coordinator', description: 'Register a new user' },
    { method: 'qed_deploy_contract', endpoint: 'coordinator', description: 'Deploy a new contract' },
    { method: 'qed_build_block', endpoint: 'coordinator', description: 'Build a new block' },
    { method: 'qed_get_contract_leaf_data', endpoint: 'coordinator', description: 'Get contract leaf data' },
    { method: 'qed_get_contract_code_definition', endpoint: 'coordinator', description: 'Get contract code definition' },
    { method: 'qed_get_user_leaf_data', endpoint: 'coordinator', description: 'Get user leaf data globally' },

    // Realm-only methods
    { method: 'qed_check_user_id_in_realm', endpoint: 'realm', description: 'Check if user belongs to this realm' },
    { method: 'qed_get_user_contract_state_tree_merkle_proof', endpoint: 'realm', description: 'Get merkle proof for user contract state' },
    { method: 'qed_get_user_contract_tree_root', endpoint: 'realm', description: 'Get user contract tree root' },
    { method: 'qed_get_user_contract_state_tree_root', endpoint: 'realm', description: 'Get user contract state tree root' },
    { method: 'qed_get_user_bottom_tree_merkle_proof', endpoint: 'realm', description: 'Get user bottom tree merkle proof' },

    // Methods available on both endpoints
    { method: 'qed_get_latest_l2_block_state', endpoint: 'both', description: 'Get latest L2 block state' },
    { method: 'qed_get_checkpoint_tree_root', endpoint: 'both', description: 'Get checkpoint tree root' },
    { method: 'qed_get_user_tree_root', endpoint: 'both', description: 'Get user tree root' },
];

/**
 * Helper function to determine which endpoint to use for a method
 */
export function getEndpointForMethod(method: string): 'coordinator' | 'realm' | 'both' {
    const methodInfo = RPC_METHOD_MAPPING.find(m => m.method === method);
    return methodInfo?.endpoint || 'coordinator';
}

/**
 * Type guards for method categorization
 */
export function isCoordinatorMethod(method: string): boolean {
    return Object.values(CoordinatorRpcMethods).includes(method as any);
}

export function isRealmMethod(method: string): boolean {
    return Object.values(RealmRpcMethods).includes(method as any);
}