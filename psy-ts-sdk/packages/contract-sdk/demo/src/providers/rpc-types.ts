// src/providers/rpc-types.ts

/**
 * RPC methods that should be called on the Coordinator endpoint
 * These handle global operations and cross-realm data
 */
export enum CoordinatorRpcMethods {
    // Checkpoint operations
    LATEST_CHECKPOINT = 'psy_latest_checkpoint',
    GET_CHECKPOINT_LEAF_DATA = 'psy_get_checkpoint_leaf_data',
    GET_CHECKPOINT_GLOBAL_STATE_ROOTS = 'psy_get_checkpoint_global_state_roots',
    GET_CHECKPOINT_TREE_ROOT = 'psy_get_checkpoint_tree_root',
    GET_CHECKPOINT_TREE_LEAF_HASH = 'psy_get_checkpoint_tree_leaf_hash',
    GET_CHECKPOINT_TREE_MERKLE_PROOF = 'psy_get_checkpoint_tree_merkle_proof',

    // Block operations
    BUILD_BLOCK = 'psy_build_block',
    GET_LATEST_BLOCK_STATE = 'psy_get_latest_block_state',
    GET_BLOCK_STATE = 'psy_get_block_state',

    // User operations
    REGISTER_USER = 'psy_register_user',
    GET_USER_LEAF_DATA = 'psy_get_user_leaf_data',
    GET_USER_TREE_ROOT = 'psy_get_user_tree_root',
    GET_USER_SUB_TREE_MERKLE_PROOF = 'psy_get_user_sub_tree_merkle_proof',
    GET_USER_TREE_MERKLE_PROOF = 'psy_get_user_tree_merkle_proof',

    // Contract operations
    DEPLOY_CONTRACT = 'psy_deploy_contract',
    GET_CONTRACT_LEAF_DATA = 'psy_get_contract_leaf_data',
    GET_CONTRACT_CODE_DEFINITION = 'psy_get_contract_code_definition',

    // Transaction submission
    SUBMIT_TRANSACTION = 'psy_submit_transaction', // This might be different in your system
}

/**
 * RPC methods that should be called on the Realm endpoint
 * These handle realm-specific state queries
 */
export enum RealmRpcMethods {
    // User state queries
    CHECK_USER_ID_IN_REALM = 'psy_check_user_id_in_realm',
    GET_USER_TREE_LEAF_HASH = 'psy_get_user_tree_leaf_hash',
    GET_USER_REGISTRATION_TREE_ROOT = 'psy_get_user_registration_tree_root',
    GET_USER_BOTTOM_TREE_MERKLE_PROOF = 'psy_get_user_bottom_tree_merkle_proof',

    // Contract state queries
    GET_USER_CONTRACT_TREE_ROOT = 'psy_get_user_contract_tree_root',
    GET_USER_CONTRACT_STATE_TREE_ROOT = 'psy_get_user_contract_state_tree_root',
    GET_USER_CONTRACT_TREE_MERKLE_PROOF = 'psy_get_user_contract_tree_merkle_proof',
    GET_USER_CONTRACT_STATE_TREE_MERKLE_PROOF = 'psy_get_user_contract_state_tree_merkle_proof',

    // Some methods might be available on both endpoints
    // These are typically replicated for performance
    GET_LATEST_BLOCK_STATE_REALM = 'psy_get_latest_block_state',
    GET_BLOCK_STATE_REALM = 'psy_get_block_state',
    GET_CHECKPOINT_LEAF_DATA_REALM = 'psy_get_checkpoint_leaf_data',
    GET_CHECKPOINT_TREE_ROOT_REALM = 'psy_get_checkpoint_tree_root',
    GET_USER_TREE_ROOT_REALM = 'psy_get_user_tree_root',
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
    { method: 'psy_latest_checkpoint', endpoint: 'coordinator', description: 'Get the latest checkpoint ID' },
    { method: 'psy_register_user', endpoint: 'coordinator', description: 'Register a new user' },
    { method: 'psy_deploy_contract', endpoint: 'coordinator', description: 'Deploy a new contract' },
    { method: 'psy_build_block', endpoint: 'coordinator', description: 'Build a new block' },
    { method: 'psy_get_contract_leaf_data', endpoint: 'coordinator', description: 'Get contract leaf data' },
    { method: 'psy_get_contract_code_definition', endpoint: 'coordinator', description: 'Get contract code definition' },
    { method: 'psy_get_user_leaf_data', endpoint: 'coordinator', description: 'Get user leaf data globally' },

    // Realm-only methods
    { method: 'psy_check_user_id_in_realm', endpoint: 'realm', description: 'Check if user belongs to this realm' },
    { method: 'psy_get_user_contract_state_tree_merkle_proof', endpoint: 'realm', description: 'Get merkle proof for user contract state' },
    { method: 'psy_get_user_contract_tree_root', endpoint: 'realm', description: 'Get user contract tree root' },
    { method: 'psy_get_user_contract_state_tree_root', endpoint: 'realm', description: 'Get user contract state tree root' },
    { method: 'psy_get_user_bottom_tree_merkle_proof', endpoint: 'realm', description: 'Get user bottom tree merkle proof' },

    // Methods available on both endpoints
    { method: 'psy_get_latest_block_state', endpoint: 'both', description: 'Get latest block state' },
    { method: 'psy_get_checkpoint_tree_root', endpoint: 'both', description: 'Get checkpoint tree root' },
    { method: 'psy_get_user_tree_root', endpoint: 'both', description: 'Get user tree root' },
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