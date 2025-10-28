import { useEffect } from 'react';
import { useWalletState } from '@qed/psy-wallet-widget';
import { QedJSON } from '@qed/psy-sdk';

export const WALLET_STORAGE_KEY = 'psy_wallet_data';

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

export interface StoredWalletData {
  wallets: {
    userId: number;
    name: string;
    address: string;
    balance: string;
    networkId: string;
    publicKeyHex: string;
    privateKey: string | null;
    signType: string;
    fingerprint: string | null;
  }[];
  activeWalletId?: number;
  lastUpdated: number;
}

export const usePersistentWallet = () => {
  const wallets = useWalletState((state) => state.wallets);
  const currentWallet = useWalletState((state) => state.currentWallet);
  const addWalletFromPrivateKey = useWalletState((state) => state.addWalletFromPrivateKey);
  const setActiveWalletAsync = useWalletState((state) => state.setActiveWalletAsync);

  // Load wallets from localStorage on mount
  useEffect(() => {
    const loadStoredWallets = async () => {
      try {
        console.log('Checking for stored wallets...');
        const stored = localStorage.getItem(WALLET_STORAGE_KEY);
        console.log('Stored data:', stored);
        
        if (stored) {
          const data: StoredWalletData = QedJSON.parse(stored);
          console.log('Parsed stored data:', data);
          
          // Check if data is not too old (24 hours)
          const isDataFresh = Date.now() - data.lastUpdated < 24 * 60 * 60 * 1000;
          console.log('Data is fresh:', isDataFresh);
          
          if (isDataFresh && data.wallets.length > 0) {
            console.log('Loading stored wallets:', data.wallets.length);
            console.log('Current wallets count:', wallets.length);
            
            if (wallets.length === 0) {
              console.log('Delayed wallet restoration starting...');
              
              // Restore wallets sequentially to avoid race conditions
              for (const walletData of data.wallets) {
                if (walletData.privateKey) {
                  try {
                    console.log('Restoring wallet:', walletData.userId, 'with private key length:', walletData.privateKey.length);
                    await addWalletFromPrivateKey(walletData.privateKey, walletData.signType, walletData.fingerprint, true, false);
                  } catch (error) {
                    console.warn('Failed to restore wallet:', walletData.userId, error);
                  }
                }
              }

              // set current wallet
              if (data.activeWalletId) {
                await setActiveWalletAsync(data.activeWalletId);
              }

              console.log('Delayed wallet restoration completed');
            } else {
              console.log('Wallets already exist, skipping delayed restoration');
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
  }, []);

  // Save wallets to localStorage whenever wallets change
  useEffect(() => {
    if (wallets.length > 0) {
      // Delay save to avoid frequent localStorage writes
      const timer = setTimeout(() => {
        const saveWallets = async () => {
        try {
          const walletsData = await Promise.all(
            wallets.map(async (wallet) => {
              try {
                // Get private key asynchronously
                const privateKey = await wallet.wallet?.signer?.getPrivateKeyHex?.();
                const signType = await wallet.wallet?.signer?.getSignType?.();
                const fingerprint = await wallet.wallet?.signer?.getFingerprint?.();
                
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
                  signType: signType || null,
                  fingerprint: fingerprint || null,
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
              walletsData[0]?.userId,
            lastUpdated: Date.now(),
          };

          // Clean all BigInt values recursively
          const cleanedData = cleanBigIntValues(dataToStore);
          
          localStorage.setItem(WALLET_STORAGE_KEY, QedJSON.stringify(cleanedData));
          console.log('Saved wallets to storage:', dataToStore.wallets.length);
        } catch (error) {
          console.warn('Failed to save wallets:', error);
        }
        };

        saveWallets();
      }, 500); // 500ms debounce
      
      return () => clearTimeout(timer);
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