import {
  DogeLinkElectrsRPC,
  DogeMemoryWalletProvider,
  DogeNetworkId,
  IAddressStatsResponse,
  IDogeTransactionSigner,
  TWalletAbility,
  getP2PKHAddressFromPublicKey,
  hexToU8Array,
} from "doge-sdk";
import { StoreApi, create } from "zustand";
import { IQWidgetWallet } from "../types";
import { WidgetDogeWalletProvider } from "../utils/provider";
import { WalletWidgetRPC } from "../utils/rpc/walletRPC";
enum WalletWidgetLoadingState {
  Loading,
  Ready,
  FatalError,
}
interface IWalletStateStore {
  loadingState: WalletWidgetLoadingState;
  provider: WidgetDogeWalletProvider<any>;
  abilities: TWalletAbility[];
  wallets: IQWidgetWallet[];
  currentWallet?: IQWidgetWallet;
  currency: string;
  rpc: WalletWidgetRPC;
  setRPC(rpc: WalletWidgetRPC): void;
  addWallet: (wallet: IQWidgetWallet) => any;
  removeWallet: (address: string) => any;
  setWallets: (wallets: IQWidgetWallet[]) => any;
  setActiveWallet: (address: string) => any;
  setActiveWalletAsync: (address: string) => Promise<any>;
  setWalletProvider: (provider: WidgetDogeWalletProvider<any>) => any;
  refreshCurrentWalletUTXOs: () => Promise<any>;
  addRandomWallet: (changeCurrent?: boolean) => Promise<any>;
  addWalletFromWIF: (wif: string, changeCurrent?: boolean) => Promise<any>;
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

async function getSignerAddress(networkId: DogeNetworkId, signer: IDogeTransactionSigner){
  const pubKey = await signer.getCompressedPublicKey();
  return getP2PKHAddressFromPublicKey(hexToU8Array(pubKey), networkId);
}
async function getWidgetWallet(signer: IDogeTransactionSigner, rpc: WalletWidgetRPC, address?: string): Promise<IQWidgetWallet>{
  const networkId = rpc.getNetwork().networkId;
  const realAddress = address || (await getSignerAddress(networkId, signer));
  const stats = (await rpc.getStatsFor(realAddress)) as IAddressStatsResponse;
  const utxos = await rpc.getUTXOs(realAddress);
  const confirmedBalance = stats.chain_stats.funded_txo_sum - stats.chain_stats.spent_txo_sum;
  const balance = confirmedBalance+(stats.mempool_stats.funded_txo_sum - stats.mempool_stats.spent_txo_sum);
  return {
    address: realAddress,
    balance,
    confirmedBalance,
    stats,
    utxos,
    networkId,
    signer,
  };

}
const useWalletState = create<IWalletStateStore>((set, get, api) => {
  const setAsync = setAsyncFactory(set, get, api);
  return {
    loadingState: WalletWidgetLoadingState.Ready,
    provider:WidgetDogeWalletProvider.fromMemoryProvider(
      new DogeMemoryWalletProvider()
    ),
    wallets: [],
    abilities:new DogeMemoryWalletProvider().getAbilities(),
    networkId: "dogeRegtest",
    canAddWallet: true,
    currency: "DOGE",
    rpc: new WalletWidgetRPC(
      "dogeRegtest",
      "http://localhost:1337/api",
      "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest"
    ),
    refreshCurrentWalletUTXOs: () =>
      setAsync(async ({ state }) => {
        //await waitMs(2000);
        const { currentWallet, rpc, wallets: currentStateWallets } = state;
        if (!currentWallet) {
          return {};
        }
        const widgetWallet= await getWidgetWallet(currentWallet.signer, rpc, currentWallet.address);
        const wallets = currentStateWallets.map((wallet) => {
          if (wallet.address === currentWallet.address) {
            return {...widgetWallet};
          } else {
            return wallet;
          }
        });

        return {
          wallets,
          currentWallet: { ...widgetWallet },
        };
      }),
    addWallet: (wallet: IQWidgetWallet) =>
      set((state) => {
        state.wallets.push(wallet);
        return { wallets: state.wallets };
      }),
    removeWallet: (address: string) =>
      set((state) => {
        state.wallets = state.wallets.filter(
          (wallet) => wallet.address !== address
        );
        if (state.currentWallet?.address === address) {
          return {
            currentWallet: state.wallets[0] || undefined,
            wallets: state.wallets,
          };
        }
        return { wallets: state.wallets };
      }),
    setWallets: (wallets: IQWidgetWallet[]) =>
      set((state) => {
        if(state.currentWallet){
          const currentWallet = wallets.find((w) => w.address === state.currentWallet?.address);
          return {
            wallets,
            currentWallet,
          };
        }
        return { wallets };
      }),
    setRPC: (rpc: WalletWidgetRPC) =>
      set((state) => {
        return { rpc };
      }),
      setActiveWalletAsync: (address: string) =>
        setAsync(async ({ state }) => {
          //await waitMs(2000);
          if(state.currentWallet?.address === address){
            return {};
          }
          const wallet = state.wallets.find(
            (wallet) => wallet.address === address
          );
          if (!wallet) {
            return {};
          }
          const widgetWallet= await getWidgetWallet(wallet.signer, state.rpc, wallet.address);
          const wallets = state.wallets.map((w) => {
            if (w.address === wallet.address) {
              return {...widgetWallet};
            } else {
              return w;
            }
          });
  
          return {
            wallets,
            currentWallet: { ...widgetWallet },
          };
        }),
    setActiveWallet: (address: string) =>
      set((state) => {
        if(state.currentWallet?.address === address){
          return {};
        }
        const wallet = state.wallets.find(
          (wallet) => wallet.address === address
        );
        if (!wallet) {
          return {};
        }
        return { currentWallet: wallet };
      }),
      
      addRandomWallet: (changeCurrent?: boolean) => setAsync(async ({ set, get, state }) => {
        if(!state.abilities.includes("add-wallet-random")){
          return {};
        }


        const provider = state.provider;
        const networkId = state.rpc.getNetwork().networkId;
        const signer = await provider.addWalletRandom(networkId);
        const compressedPublicKey = await signer.getCompressedPublicKey();
        const address = getP2PKHAddressFromPublicKey(hexToU8Array(compressedPublicKey), networkId);

        const widgetWallet = await getWidgetWallet(signer, state.rpc, address);
        
        const wallets = [...state.wallets, widgetWallet];
        if(!state.currentWallet || changeCurrent){
          return {
            wallets,
            currentWallet: {...widgetWallet},
          };
        }else{
          return {
            wallets,
          };
        }

      }),
      
      addWalletFromWIF: (wif: string, changeCurrent?: boolean) => setAsync(async ({ set, get, state }) => {
        if(!state.abilities.includes("add-wallet-bip178")){
          return {};
        }


        const provider = state.provider;
        const networkId = state.rpc.getNetwork().networkId;
        const signer = await provider.addWalletBIP178(networkId, wif);
        const compressedPublicKey = await signer.getCompressedPublicKey();
        const address = getP2PKHAddressFromPublicKey(hexToU8Array(compressedPublicKey), networkId);
        const widgetWallet = await getWidgetWallet(signer, state.rpc, address);
        const wallets = [...state.wallets, widgetWallet];
        if(!state.currentWallet || changeCurrent){
          return {
            wallets,
            currentWallet: {...widgetWallet},
          };
        }else{
          return {
            wallets,
          };
        }

      }),
    setWalletProvider: (provider: WidgetDogeWalletProvider<any>) =>
      setAsync(async ({ set, get, state }) => {
        if (provider === state.provider) {
          return {};
        }
        const networkId = state.rpc.getNetwork().networkId;
        const addresses = await provider.getP2PKHAddresses(networkId);
        const signers: IDogeTransactionSigner[] = [];
        for(const address of addresses){
          signers.push(await provider.getSignerForAddress(address.address));
        }
        const wallets: IQWidgetWallet[] = await Promise.all(addresses.map((address, i) => {
          return getWidgetWallet(signers[i], state.rpc, address.address);
        }));
        return {
          provider,
          abilities: provider.getAbilities(),
          wallets,
          currentWallet: wallets[0] || undefined,
        };
      }, true),
  };
});

export { useWalletState, WalletWidgetLoadingState };
