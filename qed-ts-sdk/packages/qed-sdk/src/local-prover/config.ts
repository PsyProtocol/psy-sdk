// RPC configuration types
export interface RpcConfig {
    users_per_realm: number;
    realm_configs: RealmRpcConfig[];
    coordinator_configs: CoordinatorRpcConfig[];
}

export interface RealmRpcConfig {
    id: number;
    rpc_url: string[];
}

export interface CoordinatorRpcConfig {
    id: number;
    rpc_url: string[];
}

// Default configuration factory function
export function createDefaultRpcConfig(): RpcConfig {
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

// Type guards for runtime type checking
export function isRpcConfig(obj: any): obj is RpcConfig {
    return (
        obj &&
        typeof obj.users_per_realm === "number" &&
        Array.isArray(obj.realm_configs) &&
        Array.isArray(obj.coordinator_configs) &&
        obj.realm_configs.every(isRealmRpcConfig) &&
        obj.coordinator_configs.every(isCoordinatorRpcConfig)
    );
}

export function isRealmRpcConfig(obj: any): obj is RealmRpcConfig {
    return (
        obj &&
        typeof obj.id === "number" &&
        Array.isArray(obj.rpc_url) &&
        obj.rpc_url.every((url: any) => typeof url === "string")
    );
}

export function isCoordinatorRpcConfig(obj: any): obj is CoordinatorRpcConfig {
    return (
        obj &&
        typeof obj.id === "number" &&
        Array.isArray(obj.rpc_url) &&
        obj.rpc_url.every((url: any) => typeof url === "string")
    );
}
