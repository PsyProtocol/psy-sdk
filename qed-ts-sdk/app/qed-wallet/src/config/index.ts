import React from 'react';

// Configuration interface for the wallet
export interface WalletConfig {
  theme: {
    colors: {
      background: string;
      text: string;
      primary: string;
      primaryText: string;
      border: string;
      accent: string;
    };
  };
  network: {
    rpcUrl: string;
    networkId: string;
    chainId: number;
    name: string;
  };
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
    },
  },
  network: {
    rpcUrl: 'http://localhost:8545',
    networkId: 'localhost',
    chainId: 1337,
    name: 'Local Network',
  },
  wallet: {
    defaultWalletName: 'Wallet 1',
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

  return {
    config,
    updateConfig,
  };
};