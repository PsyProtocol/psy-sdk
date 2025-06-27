import React from 'react';

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
  users_per_realm: number;
  realm_configs: RealmConfig[];
  coordinator_configs: CoordinatorConfig[];
  prover_url?: string;
  nativeCurrency?: string; // contractId of the native currency token
}

export const DEFAULT_PROVER_URL = "http://127.0.0.1:8888";

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

// Default configuration
export const defaultConfig: WalletConfig = {
  theme: {
    colors: {
      background: '#ffffff',
      text: '#73e7ff',
      primary: '#73e7ff',
      primaryText: '#ffffff',
      border: '#73e7ff',
      accent: '#73e7ff',
      success: '#00C851',
      error: '#ff6b6b',
      textSecondary: '#666666',
    },
  },
  network: {
    users_per_realm: 32768,
    realm_configs: [
      {
        id: 0,
        rpc_url: ["http://127.0.0.1:8546"]
      },
      {
        id: 16384,
        rpc_url: ["http://127.0.0.1:8547"]
      },
      {
        id: 8192,
        rpc_url: ["http://127.0.0.1:8548"]
      }
    ],
    coordinator_configs: [
      {
        id: 0,
        rpc_url: ["http://127.0.0.1:8545"]
      }
    ],
    prover_url: "http://127.0.0.1:8888",
    nativeCurrency: "0"
  },
  wallet: {
    defaultWalletName: '0',
    enableAutoRefresh: true,
    refreshInterval: 30000, // 30 seconds
  },
  extension: {
    width: 375,
    height: 600,
    title: 'Psy: The Internet Unchained',
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
    return config.network.prover_url || "http://127.0.0.1:8888";
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