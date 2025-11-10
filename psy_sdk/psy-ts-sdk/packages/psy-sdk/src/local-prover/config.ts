// RPC configuration types
export interface WebProverConfig {
    users_per_realm: number;
    realm_configs: RpcConfig[];
    coordinator_configs: RpcConfig[];
}

export interface RpcConfig {
    id: number;
    rpc_url: string[];
}

export function createDefaultRpcConfig(): WebProverConfig {
    return {
        users_per_realm: 32768,
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
    };
}
