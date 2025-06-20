import { useEffect } from 'react';
import { useWalletState } from '@qed/qed-wallet-widget';

const WALLET_STORAGE_KEY = 'psy_wallet_data';

interface StoredWalletData {
  wallets: any[];
  activeWalletId?: number;
  lastUpdated: number;
}

export const usePersistentWallet = () => {
  const [
    wallets,
    currentWallet,
    addWalletFromPrivateKey
  ] = useWalletState((state) => [
    state.wallets,
    state.currentWallet,
    state.addWalletFromPrivateKey
  ]);

  // Load wallets from localStorage on mount
  useEffect(() => {
    const loadStoredWallets = () => {
      try {
        const stored = localStorage.getItem(WALLET_STORAGE_KEY);
        if (stored) {
          const data: StoredWalletData = JSON.parse(stored);
          
          // Check if data is not too old (24 hours)
          const isDataFresh = Date.now() - data.lastUpdated < 24 * 60 * 60 * 1000;
          
          if (isDataFresh && data.wallets.length > 0) {
            console.log('Loading stored wallets:', data.wallets.length);
            
            // Restore wallets if they don't exist
            if (wallets.length === 0) {
              // Note: This is a simplified restoration
              // In a real implementation, you'd need to properly restore wallet instances
              data.wallets.forEach(async (walletData) => {
                if (walletData.privateKey) {
                  try {
                    await addWalletFromPrivateKey(walletData.privateKey, false, false);
                  } catch (error) {
                    console.warn('Failed to restore wallet:', error);
                  }
                }
              });
            }
          }
        }
      } catch (error) {
        console.warn('Failed to load stored wallets:', error);
      }
    };

    loadStoredWallets();
  }, []);

  // Save wallets to localStorage whenever wallets change
  useEffect(() => {
    if (wallets.length > 0) {
      try {
        const dataToStore: StoredWalletData = {
          wallets: wallets.map(wallet => ({
            userId: wallet.userId,
            name: wallet.name || `Wallet ${wallet.userId}`,
            address: wallet.address,
            balance: wallet.balance,
            networkId: wallet.networkId,
            // Note: In production, you should encrypt private keys or use a more secure storage method
            privateKey: wallet.wallet?.signer?.getPrivateKeyHex?.() || null,
          })),
          activeWalletId: currentWallet?.userId,
          lastUpdated: Date.now(),
        };

        localStorage.setItem(WALLET_STORAGE_KEY, JSON.stringify(dataToStore));
        console.log('Saved wallets to storage:', dataToStore.wallets.length);
      } catch (error) {
        console.warn('Failed to save wallets:', error);
      }
    }
  }, [wallets, currentWallet]);

  // Clear stored data (useful for logout or reset)
  const clearStoredWallets = () => {
    localStorage.removeItem(WALLET_STORAGE_KEY);
  };

  return {
    clearStoredWallets,
  };
};