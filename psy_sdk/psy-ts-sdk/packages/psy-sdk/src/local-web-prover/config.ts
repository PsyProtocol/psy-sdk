// RPC configuration types
export interface WebProverConfig {
    global_user_tree_height: number;
    realm_user_tree_height: number;
    users_per_realm: number;
    realm_configs: RpcConfig[];
    coordinator_configs: RpcConfig[];
    prove_proxy_url: string[];
}

interface RpcConfig {
    id: number;
    rpc_url: string[];
}

// Default configuration factory function
export function createDefaultRpcConfig(): WebProverConfig {
    return {
        users_per_realm: 8388608,
        global_user_tree_height: 24,
        realm_user_tree_height: 23,
        realm_configs: [
            {
                id: 0,
                rpc_url: ["http://127.0.0.1:8546"],
            },
            {
                id: 16384,
                rpc_url: ["http://127.0.0.1:8547"],
            },
            {
                id: 8192,
                rpc_url: ["http://127.0.0.1:8548"],
            },
        ],
        coordinator_configs: [
            {
                id: 0,
                rpc_url: ["http://127.0.0.1:8545"],
            },
        ],
        prove_proxy_url: ["http://127.0.0.1:9999"],
    };
}
