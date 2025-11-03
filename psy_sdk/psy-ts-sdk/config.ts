import * as rootConfig from '../../config.json';

export interface PsyConfig {
  network: {
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
    native_currency: string;
    native_currency_decimal: number;
    native_currency_name: string;
    fees: {
      register_user_fee: number;
      deploy_contract_fee: number;
      guta_fee: number;
    };
  };
  genesis: {
    precompiles: Array<{
      name: string;
      path: string;
      contract_name: string;
      method_names: string[];
      deployer: string;
    }>;
    users: Array<{
      public_key_param: string;
      fingerprint: string;
    }>;
    contracts: Record<string, Record<string, { slots: Record<string, string> }>>;
  };
  wallet: {
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
  };
  whitelist: {
    enabled: boolean;
    secp256k1: string[];
  };
}

// Cast the imported config to our typed interface
export const psyConfig = rootConfig as PsyConfig;

// Export commonly used configuration sections
export const networkConfig = psyConfig.network;
export const walletConfig = psyConfig.wallet;
export const genesisConfig = psyConfig.genesis;
export const whitelistConfig = psyConfig.whitelist;

// Helper functions for common config access patterns
export function getCoordinatorRpcUrl(): string {
  if (networkConfig.coordinator_configs.length > 0 && networkConfig.coordinator_configs[0].rpc_url.length > 0) {
    return networkConfig.coordinator_configs[0].rpc_url[0];
  }
  return "http://127.0.0.1:8545"; // fallback
}

export function getRealmRpcUrl(realmId?: number): string {
  const targetRealm = realmId !== undefined
    ? networkConfig.realm_configs.find(r => r.id === realmId)
    : networkConfig.realm_configs[0];

  if (targetRealm && targetRealm.rpc_url.length > 0) {
    return targetRealm.rpc_url[0];
  }
  return "http://127.0.0.1:8546"; // fallback
}

export function getProveProxyUrl(): string {
  if (networkConfig.prove_proxy_url.length > 0) {
    return networkConfig.prove_proxy_url[0];
  }
  return "http://127.0.0.1:9999"; // fallback
}

export function getNativeCurrency(): string {
  return networkConfig.native_currency || "0";
}

export function getNativeCurrencyName(): string {
  return networkConfig.native_currency_name || "PSY";
}

export function getNativeCurrencyDecimals(): number {
  return networkConfig.native_currency_decimal || 9;
}

// Export the raw config for backward compatibility
export default psyConfig;
