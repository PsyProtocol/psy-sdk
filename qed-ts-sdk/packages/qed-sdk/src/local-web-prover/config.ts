// RPC configuration types
export interface WebProverConfig {
    users_per_realm: number;
    realm_configs: RpcConfig[];
    coordinator_configs: RpcConfig[];
}

interface RpcConfig {
    id: number;
    rpc_url: string[];
}

// Default configuration factory function
export function createDefaultRpcConfig(): WebProverConfig {
    // const REALM_USER_TREE_HEIGHT = 15; // You may need to adjust this value based on your constants
    // const users_per_realm = 1 << REALM_USER_TREE_HEIGHT;
    // return {
    //     users_per_realm: users_per_realm,
    //     realm_configs: [
    //         {
    //             id: 0,
    //             rpc_url: ["http://127.0.0.1:8546"],
    //         },
    //         {
    //             id: 16384,
    //             rpc_url: ["http://127.0.0.1:8547"],
    //         },
    //     ],
    //     coordinator_configs: [
    //         {
    //             id: 0,
    //             rpc_url: ["http://127.0.0.1:8545"],
    //         },
    //     ],
    // };
    //
    return {
        "users_per_realm": 32768,
        "realm_configs": [
            {
                "id": 0,
                "rpc_url": [
                    "http://127.0.0.1:8546"
                ]
            },
            {
                "id": 16384,
                "rpc_url": [
                    "http://127.0.0.1:8547"
                ]
            },
            {
                "id": 8192,
                "rpc_url": [
                    "http://127.0.0.1:8548"
                ]
            }
        ],
        "coordinator_configs": [
            {
                "id": 0,
                "rpc_url": [
                    "http://127.0.0.1:8545"
                ]
            }
        ]
    }
}

