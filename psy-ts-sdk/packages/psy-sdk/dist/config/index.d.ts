export interface PsyNetworkConfig {
    magic: string;
    users_per_realm: number;
    global_user_tree_height: number;
    realm_user_tree_height: number;
    group_realm_height: number;
    realm_configs: Array<{
        id: number;
        rpc_url: string[];
    }>;
    coordinator_configs: Array<{
        id: number;
        rpc_url: string[];
    }>;
    prove_proxy_url: string[];
    api_services_url?: string[];
    native_currency: string;
    native_currency_decimal: number;
    native_currency_name: string;
    fees: {
        register_user_fee: number;
        deploy_contract_fee: number;
        guta_fee: number;
    };
    genesis?: PsyGenesisConfig;
    wallet?: PsyWalletConfig;
    whitelist?: PsyWhitelistConfig;
}
export interface PsyGenesisConfig {
    precompiles: Array<{
        name: string;
        deployer: string;
        bytecode: any[];
    }>;
    users: Array<{
        public_key_param: string;
        fingerprint: string;
    }>;
    contracts: Record<string, Record<string, {
        slots: Record<string, string>;
    }>>;
}
export interface PsyWalletConfig {
    default_wallet_name: string;
    enable_auto_refresh: boolean;
    refresh_interval: number;
    theme: {
        colors: {
            background: string;
            text: string;
            primary: string;
            primary_text: string;
            border: string;
            accent: string;
            success: string;
            error: string;
            text_secondary: string;
        };
    };
    extension: {
        width: number;
        height: number;
        title: string;
    };
}
export interface PsyWhitelistConfig {
    enabled: boolean;
    secp256k1: string[];
}
export interface PsyConfig {
    networks: Record<string, PsyNetworkConfig>;
    defaultNetwork: string;
}
//# sourceMappingURL=index.d.ts.map