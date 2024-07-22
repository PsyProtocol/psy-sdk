
import { StoreApi, create } from "zustand";
import { IQCityWidgetWallet } from "../types";
import { CityUserWalletProvider, ICityRPCProvider, ICityUserProverProvider, TCityTransactionSignerAbility, TCityTransactionSignerProviderAbility } from "@qstudio/city-sdk";
import { createMemoryWalletProvider } from "../utils/provider";
import { reverseHexBytes } from "packages/city-sdk/src/utils/felt";
enum WalletWidgetLoadingState {
  Loading,
  Ready,
  FatalError,
}
interface IWalletStateStore {
  loadingState: WalletWidgetLoadingState;
  provider: CityUserWalletProvider;
  providerAbilities: TCityTransactionSignerProviderAbility[];
  walletAbilities: TCityTransactionSignerAbility[];
  wallets: IQCityWidgetWallet[];
  currentWallet?: IQCityWidgetWallet;
  currency: string;
  rpc: ICityRPCProvider;
  setRPC(rpc: ICityRPCProvider): void;
  addWallet: (wallet: IQCityWidgetWallet) => any;
  removeWallet: (userId: number) => any;
  setWallets: (wallets: IQCityWidgetWallet[]) => any;
  setActiveWallet: (userId: number) => any;
  setActiveWalletAsync: (userId: number) => Promise<any>;
  setWalletProvider: (provider: CityUserWalletProvider) => any;
  refreshCurrentWallet: () => Promise<any>;
  refreshAllWallets: () => Promise<any>;
  addRandomWallet: (registerUser?: boolean) => Promise<any>;
  addWalletFromPrivateKey: (privateKeyHex: string, registerUser?: boolean, changeCurrent?: boolean) => Promise<any>;
}
type Setter = (
  partial:
    | IWalletStateStore
    | Partial<IWalletStateStore>
    | ((
        state: IWalletStateStore
      ) => IWalletStateStore | Partial<IWalletStateStore>),
  replace?: boolean | undefined
) => void;
type Getter = () => IWalletStateStore;
type WidgetStoreAPI = StoreApi<IWalletStateStore>;
//type AsyncWidgetStoreAction = (set: Setter, get: Getter, storeApi: WidgetStoreAPI) => Promise<void>;
type AsyncWidgetStoreAction = (helpers: {
  set: Setter;
  get: Getter;
  state: IWalletStateStore;
  storeApi: WidgetStoreAPI;
}) => Promise<Partial<IWalletStateStore> | IWalletStateStore>;


async function getAllIQWallets(provider: CityUserWalletProvider): Promise<IQCityWidgetWallet[]> {
  const users = await provider.getUserWallets();
  return Promise.all(users.map(async (user) => {
    const userInfo = await user.getUserInfo();
    return {
      ...userInfo,
      wallet: user,
    };
  }));

}
function setAsyncFactory(set: Setter, get: Getter, api: WidgetStoreAPI) {
  return async (action: AsyncWidgetStoreAction, globalAction = false) => {
    if (globalAction) {
      set({ loadingState: WalletWidgetLoadingState.Loading });
      try {
        const result = await action({
          set,
          get,
          state: get(),
          storeApi: useWalletState,
        });
        set({ ...result, loadingState: WalletWidgetLoadingState.Ready });
      } catch (err) {
        set({ loadingState: WalletWidgetLoadingState.FatalError });
      }
    } else {
      try {
        const result = await action({
          set,
          get,
          state: get(),
          storeApi: useWalletState,
        });
        set(result);
      } catch (err) {
        console.error("[WalletStateStore] unhandled error: ", err);
      }
    }
  };
}
function waitMs(duration: number){
  return new Promise((resolve)=>{
    setTimeout(resolve, duration);
  });
}
const useWalletState = create<IWalletStateStore>((set, get, api) => {
  const setAsync = setAsyncFactory(set, get, api);
  const walletProvider = createMemoryWalletProvider("http://localhost:3000?networkId=dogeRegtest", "http://localhost:1447")
  return {
    loadingState: WalletWidgetLoadingState.Ready,
    provider: walletProvider,
    wallets: [],
    providerAbilities:walletProvider.signerProvider.getAbilities(),
    walletAbilities: [],
    networkId: "dogeRegtest",
    canAddWallet: true,
    currency: "DOGE",
    rpc: walletProvider.rpc,
    refreshCurrentWallet: () =>
      setAsync(async ({ state }) => {
        //await waitMs(2000);
        const { currentWallet, rpc, wallets: currentStateWallets } = state;
        if (!currentWallet) {
          return {};
        }
        const userInfo = await currentWallet.wallet.getUserInfo();

        const wallets = currentStateWallets.map((wallet) => {
          if (wallet.userId === currentWallet.userId) {
            return {...userInfo, wallet: currentWallet.wallet};
          } else {
            return wallet;
          }
        });

        return {
          wallets,
          currentWallet: {...userInfo, wallet: currentWallet.wallet},
        };
      }),
      refreshAllWallets: () =>
        setAsync(async ({ state }) => {
          //await waitMs(2000);
          const wallets = await getAllIQWallets(state.provider);
          if(!state.currentWallet){
            return {
              wallets: wallets,
              currentWallet: wallets[0],
              walletAbilities: wallets[0]?wallets[0].wallet.signer.getAbilities():[],
            };
          }else{
            const currentWalletId = state.currentWallet?.userId;
            const currentWallet = (wallets.find((w) => w.userId === currentWalletId));
            if(currentWallet){
              return {
                wallets,
                currentWallet: currentWallet,
                walletAbilities: currentWallet.wallet.signer.getAbilities(),
              };
            }else{
              return {
                wallets,
                currentWallet: wallets[0],
                walletAbilities: wallets[0]?wallets[0].wallet.signer.getAbilities():[],
              };              
            }
          }
        }),
    addWallet: (wallet: IQCityWidgetWallet) =>
      set((state) => {
        state.wallets.push(wallet);
        return { wallets: state.wallets };
      }),
    removeWallet: (userId: number) =>
      set((state) => {
        state.wallets = state.wallets.filter(
          (wallet) => wallet.userId !== userId
        );
        if (state.currentWallet?.userId === userId) {
          return {
            currentWallet: state.wallets[0] || undefined,
            wallets: state.wallets,
          };
        }
        return { wallets: state.wallets };
      }),
    setWallets: (wallets: IQCityWidgetWallet[]) =>
      set((state) => {
        if(state.currentWallet){
          const currentWallet = wallets.find((w) => w.userId === state.currentWallet?.userId);
          return {
            wallets,
            currentWallet,
          };
        }
        return { wallets };
      }),
    setRPC: (rpc: ICityRPCProvider) =>
      set((state) => {
        return { rpc };
      }),
      setActiveWalletAsync: (userId: number) =>
        setAsync(async ({ state }) => {
          //await waitMs(2000);
          if(state.currentWallet?.userId === userId){
            return {};
          }
          const wallet = state.wallets.find(
            (wallet) => wallet.userId === userId
          );
          if (!wallet) {
            return {};
          }
          const userInfo = await wallet.wallet.getUserInfo();
  
          const wallets = state.wallets.map((w) => {
            if (w.userId === wallet.userId) {
              return {...userInfo, wallet: wallet.wallet};
            } else {
              return w;
            }
          });
  
          return {
            wallets,
            currentWallet: {...userInfo, wallet: wallet.wallet},
          };
        }),
    setActiveWallet: (userId: number) =>
      set((state) => {
        if(state.currentWallet?.userId === userId){
          return {};
        }
        const wallet = state.wallets.find(
          (wallet) => wallet.userId === userId
        );
        if (!wallet) {
          return {};
        }
        return { currentWallet: wallet };
      }),
      
      addRandomWallet: (registerUser?: boolean) => setAsync(async ({ set, get, state }) => {
        if(!state.providerAbilities.includes("add-random-private-key") || typeof state.provider.signerProvider.addRandomPrivateKey !== 'function'){
          return {};
        }


        const signer = await state.provider.signerProvider.addRandomPrivateKey();
        if(registerUser){
          const publicKeyHex = (await signer.getPublicKeyHex());
          await state.rpc.registerUser({public_key: publicKeyHex});
        }
        return {};

      }),
      
      addWalletFromPrivateKey: (privateKeyHex: string, registerUser?: boolean, changeCurrent?: boolean) => setAsync(async ({ set, get, state }) => {
        if(!state.providerAbilities.includes("import-private-key") || typeof state.provider.signerProvider.importPrivateKey !== 'function'){
          return {};
        }


      


        const signer = await state.provider.signerProvider.importPrivateKey(privateKeyHex);
        const publicKeyHex = await signer.getPublicKeyHex();

        if(registerUser){
          await state.rpc.registerUser({public_key: (publicKeyHex)});
        }

        const iqWallets = await getAllIQWallets(state.provider);

        if(changeCurrent){
          const wallet = iqWallets.filter(x=>x.publicKeyHex === publicKeyHex)[0];
          if(wallet){
            return {
              wallets: iqWallets,
              currentWallet: wallet,
            };
          }

        }
        return {
          wallets: iqWallets,
        };
      }),
    setWalletProvider: (provider: CityUserWalletProvider) =>
      setAsync(async ({ set, get, state }) => {
        if (provider === state.provider) {
          return {};
        }
        const wallets = await getAllIQWallets(provider);
        return {
          provider,
          signerAbilities: provider.signerProvider.getAbilities(),
          walletAbilities: (wallets[0]?wallets[0].wallet.signer.getAbilities():[]),
          wallets,
          networkId: provider.networkId,
          rpc: provider.rpc,
          currentWallet: wallets[0] || undefined,
        };
      }, true),
  };
});

export { useWalletState, WalletWidgetLoadingState };
