import { useEffect } from 'react';
import { useWalletState } from '@qed/qed-wallet-widget';

const WALLET_STORAGE_KEY = 'psy_wallet_data';

// Helper function to recursively clean BigInt values from objects
const cleanBigIntValues = (obj: any): any => {
  if (obj === null || obj === undefined) {
    return obj;
  }
  
  if (typeof obj === 'bigint') {
    return obj.toString();
  }
  
  if (Array.isArray(obj)) {
    return obj.map(cleanBigIntValues);
  }
  
  if (typeof obj === 'object') {
    const cleaned: any = {};
    for (const [key, value] of Object.entries(obj)) {
      cleaned[key] = cleanBigIntValues(value);
    }
    return cleaned;
  }
  
  return obj;
};

interface StoredWalletData {
  wallets: {
    userId: number;
    name: string;
    address: string;
    balance: string;
    networkId: string;
    publicKeyHex: string;
    privateKey: string | null;
  }[];
  activeWalletId?: number;
  lastUpdated: number;
}

export const usePersistentWallet = () => {
  const [
    wallets,
    currentWallet,
    setActiveWalletAsync,
    addWalletFromPrivateKey
  ] = useWalletState((state) => [
    state.wallets,
    state.currentWallet,
    state.setActiveWalletAsync,
    state.addWalletFromPrivateKey
  ]);

  // Load wallets from localStorage on mount
  useEffect(() => {
    const loadStoredWallets = async () => {
      try {
        console.log('Checking for stored wallets...');
        const stored = localStorage.getItem(WALLET_STORAGE_KEY);
        console.log('Stored data:', stored);
        
        if (stored) {
          const data: StoredWalletData = JSON.parse(stored);
          console.log('Parsed stored data:', data);
          
          // Check if data is not too old (24 hours)
          const isDataFresh = Date.now() - data.lastUpdated < 24 * 60 * 60 * 1000;
          console.log('Data is fresh:', isDataFresh);
          
          if (isDataFresh && data.wallets.length > 0) {
            console.log('Loading stored wallets:', data.wallets.length);
            console.log('Current wallets count:', wallets.length);
            
            // Always try to restore if we have stored wallets but no current wallets
            if (wallets.length === 0) {
              console.log('Restoring wallets from storage...');
              
              // Restore wallets sequentially to avoid race conditions
              for (const walletData of data.wallets) {
                if (walletData.privateKey) {
                  try {
                    console.log('Restoring wallet:', walletData.userId, 'with private key length:', walletData.privateKey.length);
                    await addWalletFromPrivateKey(walletData.privateKey, true, false);
                  } catch (error) {
                    console.warn('Failed to restore wallet:', walletData.userId, error);
                  }
                }
              }

              // set current wallet
              if (data.activeWalletId) {
                await setActiveWalletAsync(data.activeWalletId);
              }

              console.log('Wallet restoration completed');
            } else {
              console.log('Wallets already exist, skipping restoration');
            }
          } else {
            console.log('No valid stored wallets found');
          }
        } else {
          console.log('No stored wallet data found');
        }
      } catch (error) {
        console.warn('Failed to load stored wallets:', error);
      }
    };

    // Add a small delay to allow wallet state to initialize
    const timer = setTimeout(() => {
      loadStoredWallets();
    }, 100);

    return () => clearTimeout(timer);
  }, [addWalletFromPrivateKey]);

  // Save wallets to localStorage whenever wallets change
  useEffect(() => {
    if (wallets.length > 0) {
      const saveWallets = async () => {
        try {
          const walletsData = await Promise.all(
            wallets.map(async (wallet) => {
              try {
                // Get private key asynchronously
                const privateKey = await wallet.wallet?.signer?.getPrivateKeyHex?.();
                
                // Clean the data to remove any BigInt or non-serializable values
                const cleanWalletData = {
                  userId: typeof wallet.userId === 'bigint' ? Number(wallet.userId) : wallet.userId,
                  name: wallet.name || `${wallet.userId}`,
                  address: wallet.address || wallet.publicKeyHex,
                  balance: typeof wallet.balance === 'bigint' ? wallet.balance.toString() : wallet.balance,
                  networkId: wallet.networkId,
                  publicKeyHex: wallet.publicKeyHex,
                  // Note: In production, you should encrypt private keys or use a more secure storage method
                  privateKey: privateKey || null,
                };
                
                return cleanWalletData;
              } catch (error) {
                console.warn('Failed to get private key for wallet:', wallet.userId, error);
                // Clean the fallback data too
                return {
                  userId: typeof wallet.userId === 'bigint' ? Number(wallet.userId) : wallet.userId,
                  name: wallet.name || `${wallet.userId}`,
                  address: wallet.address || wallet.publicKeyHex,
                  balance: typeof wallet.balance === 'bigint' ? wallet.balance.toString() : wallet.balance,
                  networkId: wallet.networkId,
                  publicKeyHex: wallet.publicKeyHex,
                  privateKey: null,
                };
              }
            })
          );

          const dataToStore: StoredWalletData = {
            wallets: walletsData,
            activeWalletId: currentWallet?.userId ? 
              (typeof currentWallet.userId === 'bigint' ? Number(currentWallet.userId) : currentWallet.userId) : 
              undefined,
            lastUpdated: Date.now(),
          };

          // Clean all BigInt values recursively
          const cleanedData = cleanBigIntValues(dataToStore);
          
          localStorage.setItem(WALLET_STORAGE_KEY, JSON.stringify(cleanedData));
          console.log('Saved wallets to storage:', dataToStore.wallets.length);
        } catch (error) {
          console.warn('Failed to save wallets:', error);
        }
      };

      saveWallets();
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