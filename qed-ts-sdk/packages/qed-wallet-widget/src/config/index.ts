import React from 'react';
import * as rootConfig from '../../../../../config.json';

// Configuration interfaces
export interface RealmConfig {
  id: number;
  rpc_url: string[];
}

export interface CoordinatorConfig {
  id: number;
  rpc_url: string[];
}

export interface NetworkConfig {
  global_user_tree_height: number;
  realm_user_tree_height: number;
  users_per_realm: number;
  realm_configs: RealmConfig[];
  coordinator_configs: CoordinatorConfig[];
  prover_url?: string;
  nativeCurrency?: string; // contractId of the native currency token
  prove_proxy_url?: string,
}

export const DEFAULT_PROVER_URL = "http://127.0.0.1:8888";
export const DEFAULT_PROVE_PROXY_URL = "http://127.0.0.1:9999";
export const DEFAULT_GLOBAL_USER_TREE_HEIGHT = 24;
export const DEFAULT_REALM_USER_TREE_HEIGHT = 23;

export interface WalletConfig {
  theme: {
    colors: {
      background: string;
      text: string;
      primary: string;
      primaryText: string;
      border: string;
      accent: string;
      success?: string;
      error?: string;
      textSecondary?: string;
    };
  };
  network: NetworkConfig;
  wallet: {
    defaultWalletName: string;
    enableAutoRefresh: boolean;
    refreshInterval: number; // in milliseconds
  };
  extension: {
    width: number;
    height: number;
    title: string;
  };
}

// Default configuration from root config.json
export const defaultConfig: WalletConfig = {
  theme: {
    colors: {
      background: rootConfig.wallet.theme.colors.background,
      text: rootConfig.wallet.theme.colors.text,
      primary: rootConfig.wallet.theme.colors.primary,
      primaryText: rootConfig.wallet.theme.colors.primary_text,
      border: rootConfig.wallet.theme.colors.border,
      accent: rootConfig.wallet.theme.colors.accent,
      success: rootConfig.wallet.theme.colors.success,
      error: rootConfig.wallet.theme.colors.error,
      textSecondary: rootConfig.wallet.theme.colors.text_secondary,
    },
  },
  network: {
    users_per_realm: rootConfig.network.users_per_realm,
    realm_configs: rootConfig.network.realm_configs,
    coordinator_configs: rootConfig.network.coordinator_configs,
    prover_url: rootConfig.network.prover_url,
    nativeCurrency: rootConfig.network.native_currency,
  },
  wallet: {
    defaultWalletName: rootConfig.wallet.default_wallet_name,
    enableAutoRefresh: rootConfig.wallet.enable_auto_refresh,
    refreshInterval: rootConfig.wallet.refresh_interval,
  },
  extension: {
    width: rootConfig.wallet.extension.width,
    height: rootConfig.wallet.extension.height,
    title: rootConfig.wallet.extension.title,
  },
};

// Configuration storage key
const CONFIG_STORAGE_KEY = 'psy_wallet_config';

// Load configuration from localStorage
export const loadConfig = (): WalletConfig => {
  try {
    const saved = localStorage.getItem(CONFIG_STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      // Merge with default config to ensure all properties exist
      return {
        ...defaultConfig,
        ...parsed,
        theme: {
          ...defaultConfig.theme,
          ...parsed.theme,
          colors: {
            ...defaultConfig.theme.colors,
            ...parsed.theme?.colors,
          },
        },
        network: {
          ...defaultConfig.network,
          ...parsed.network,
          realm_configs: parsed.network?.realm_configs || defaultConfig.network.realm_configs,
          coordinator_configs: parsed.network?.coordinator_configs || defaultConfig.network.coordinator_configs,
          nativeCurrency: parsed.network?.nativeCurrency || defaultConfig.network.nativeCurrency,
        },
        wallet: {
          ...defaultConfig.wallet,
          ...parsed.wallet,
        },
        extension: {
          ...defaultConfig.extension,
          ...parsed.extension,
        },
      };
    }
  } catch (error) {
    console.warn('Failed to load wallet config:', error);
  }
  return defaultConfig;
};

// Save configuration to localStorage
export const saveConfig = (config: WalletConfig): void => {
  try {
    localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(config));
  } catch (error) {
    console.error('Failed to save wallet config:', error);
  }
};

// Configuration hook for React components
export const useWalletConfig = () => {
  const [config, setConfigState] = React.useState<WalletConfig>(loadConfig);

  const updateConfig = (newConfig: Partial<WalletConfig>) => {
    const updatedConfig = {
      ...config,
      ...newConfig,
      theme: {
        ...config.theme,
        ...newConfig.theme,
        colors: {
          ...config.theme.colors,
          ...newConfig.theme?.colors,
        },
      },
      network: {
        ...config.network,
        ...newConfig.network,
      },
      wallet: {
        ...config.wallet,
        ...newConfig.wallet,
      },
      extension: {
        ...config.extension,
        ...newConfig.extension,
      },
    };
    setConfigState(updatedConfig);
    saveConfig(updatedConfig);
  };

  const getCoordinatorUrl = () => {
    if (config.network.coordinator_configs.length > 0 && config.network.coordinator_configs[0].rpc_url.length > 0) {
      return config.network.coordinator_configs[0].rpc_url[0];
    }
    return "http://127.0.0.1:8545"; // fallback
  };

  const getRealmUrl = (realmId?: number) => {
    // If no specific realm ID provided, use the first one
    const targetRealm = realmId !== undefined 
      ? config.network.realm_configs.find(r => r.id === realmId)
      : config.network.realm_configs[0];
    
    if (targetRealm && targetRealm.rpc_url.length > 0) {
      return targetRealm.rpc_url[0];
    }
    return "http://127.0.0.1:8546"; // fallback
  };

  const getProverUrl = () => {
    return config.network.prover_url;
  };

  const getNativeCurrency = () => {
    return config.network.nativeCurrency || "0";
  };

  return {
    config,
    updateConfig,
    getCoordinatorUrl,
    getRealmUrl,
    getProverUrl,
    getNativeCurrency,
  };
};