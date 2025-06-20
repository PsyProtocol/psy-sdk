import { StoreApi, create } from "zustand";
import { IQedWidgetWallet } from "../types";
import {
    TQedTransactionSignerAbility,
    TQedTransactionSignerProviderAbility,
    ICoordinatorEdgeRpcProvider,
    IRealmEdgeRpcProvider,
    IQedUserWallet,
} from "@qed/qed-sdk";
import { createMemoryWalletProvider } from "../utils/provider";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";

enum WalletWidgetLoadingState {
    Loading,
    Ready,
    FatalError,
}
interface IWalletStateStore {
    loadingState: WalletWidgetLoadingState;
    provider: QedUserWalletProvider;
    providerAbilities: TQedTransactionSignerProviderAbility[];
    walletAbilities: TQedTransactionSignerAbility[];
    wallets: IQedWidgetWallet[];
    currentWallet?: IQedWidgetWallet;
    currency: string;
    // rpc: IQedRPCProvider;
    coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider;
    realmEdgeRpcProvider: IRealmEdgeRpcProvider;
    setRPC(coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider, realmEdgeRpcProvider: IRealmEdgeRpcProvider): void;
    addWallet: (wallet: IQedWidgetWallet) => any;
    removeWallet: (userId: number) => any;
    setWallets: (wallets: IQedWidgetWallet[]) => any;
    setActiveWallet: (userId: number) => any;
    setActiveWalletAsync: (userId: number) => Promise<any>;
    setWalletProvider: (provider: QedUserWalletProvider) => any;
    refreshCurrentWallet: () => Promise<any>;
    refreshAllWallets: () => Promise<any>;
    addRandomWallet: (registerUser?: boolean) => Promise<any>;
    addWalletFromPrivateKey: (privateKeyHex: string, registerUser?: boolean, changeCurrent?: boolean) => Promise<any>;
}
type Setter = (
    partial:
        | IWalletStateStore
        | Partial<IWalletStateStore>
        | ((state: IWalletStateStore) => IWalletStateStore | Partial<IWalletStateStore>),
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

async function getAllIQWallets(provider: QedUserWalletProvider): Promise<IQedWidgetWallet[]> {
    const users = await provider.getUserWallets();
    return Promise.all(
        users.map(async (user: IQedUserWallet) => {
            const userInfo = await user.getUserInfo();
            return {
                ...userInfo,
                wallet: user,
            };
        })
    );
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
function waitMs(duration: number) {
    return new Promise((resolve) => {
        setTimeout(resolve, duration);
    });
}
const useWalletState = create<IWalletStateStore>((set, get, api) => {
    const setAsync = setAsyncFactory(set, get, api);
    const walletProvider = createMemoryWalletProvider(
        "http://localhost:8545",
        "http://localhost:8546",
        "http://localhost:8888",
    );
    return {
        loadingState: WalletWidgetLoadingState.Ready,
        provider: walletProvider,
        wallets: [],
        providerAbilities: walletProvider.signerProvider.getAbilities(),
        walletAbilities: [],
        networkId: "regtest",
        canAddWallet: true,
        currency: "PSY",
        coordinatorEdgeRpcProvider: walletProvider.coordinatorEdgeRpcProvider,
        realmEdgeRpcProvider: walletProvider.realmEdgeRpcProvider,
        refreshCurrentWallet: () =>
            setAsync(async ({ state }) => {
                //await waitMs(2000);
                const {
                    currentWallet,
                    coordinatorEdgeRpcProvider,
                    realmEdgeRpcProvider,
                    wallets: currentStateWallets,
                } = state;
                if (!currentWallet) {
                    return {};
                }
                const userInfo = await currentWallet.wallet.getUserInfo();

                const wallets = currentStateWallets.map((wallet) => {
                    if (wallet.userId === currentWallet.userId) {
                        return { ...userInfo, wallet: currentWallet.wallet };
                    } else {
                        return wallet;
                    }
                });

                return {
                    wallets,
                    currentWallet: { ...userInfo, wallet: currentWallet.wallet },
                };
            }),
        refreshAllWallets: () =>
            setAsync(async ({ state }) => {
                //await waitMs(2000);
                const wallets = await getAllIQWallets(state.provider);
                if (!state.currentWallet) {
                    return {
                        wallets: wallets,
                        currentWallet: wallets[0],
                        walletAbilities: wallets[0] ? wallets[0].wallet.signer.getAbilities() : [],
                    };
                } else {
                    const currentWalletId = state.currentWallet?.userId;
                    const currentWallet = wallets.find((w) => w.userId === currentWalletId);
                    if (currentWallet) {
                        return {
                            wallets,
                            currentWallet: currentWallet,
                            walletAbilities: currentWallet.wallet.signer.getAbilities(),
                        };
                    } else {
                        return {
                            wallets,
                            currentWallet: wallets[0],
                            walletAbilities: wallets[0] ? wallets[0].wallet.signer.getAbilities() : [],
                        };
                    }
                }
            }),
        addWallet: (wallet: IQedWidgetWallet) =>
            set((state) => {
                state.wallets.push(wallet);
                return { wallets: state.wallets };
            }),
        removeWallet: (userId: number) =>
            set((state) => {
                state.wallets = state.wallets.filter((wallet) => wallet.userId !== userId);
                if (state.currentWallet?.userId === userId) {
                    return {
                        currentWallet: state.wallets[0] || undefined,
                        wallets: state.wallets,
                    };
                }
                return { wallets: state.wallets };
            }),
        setWallets: (wallets: IQedWidgetWallet[]) =>
            set((state) => {
                if (state.currentWallet) {
                    const currentWallet = wallets.find((w) => w.userId === state.currentWallet?.userId);
                    return {
                        wallets,
                        currentWallet,
                    };
                }
                return { wallets };
            }),
        setRPC: (coordinatorEdgeRpcProvider, realmEdgeRpcProvider) =>
            set((state) => {
                return { coordinatorEdgeRpcProvider, realmEdgeRpcProvider };
            }),
        setActiveWalletAsync: (userId: number) =>
            setAsync(async ({ state }) => {
                //await waitMs(2000);
                if (state.currentWallet?.userId === userId) {
                    return {};
                }
                const wallet = state.wallets.find((wallet) => wallet.userId === userId);
                if (!wallet) {
                    return {};
                }
                const userInfo = await wallet.wallet.getUserInfo();

                const wallets = state.wallets.map((w) => {
                    if (w.userId === wallet.userId) {
                        return { ...userInfo, wallet: wallet.wallet };
                    } else {
                        return w;
                    }
                });

                return {
                    wallets,
                    currentWallet: { ...userInfo, wallet: wallet.wallet },
                };
            }),
        setActiveWallet: (userId: number) =>
            set((state) => {
                if (state.currentWallet?.userId === userId) {
                    return {};
                }
                const wallet = state.wallets.find((wallet) => wallet.userId === userId);
                if (!wallet) {
                    return {};
                }
                return { currentWallet: wallet };
            }),

        addRandomWallet: (registerUser?: boolean) =>
            setAsync(async ({ set, get, state }) => {
                if (
                    !state.providerAbilities.includes("add-random-private-key") ||
                    typeof state.provider.signerProvider.addRandomPrivateKey !== "function"
                ) {
                    return {};
                }

                const signer = await state.provider.signerProvider.addRandomPrivateKey();
                if (registerUser) {
                    if (typeof signer.getPrivateKeyHex !== "function") {
                        return {};
                    }
                    const privateKeyHex = await signer.getPrivateKeyHex();
                    await state.provider.signerProvider.registerUser(privateKeyHex);
                }
                return {};
            }),

        addWalletFromPrivateKey: (privateKeyHex: string, registerUser?: boolean, changeCurrent?: boolean) =>
            setAsync(async ({ set, get, state }) => {
                if (
                    !state.providerAbilities.includes("import-private-key") ||
                    typeof state.provider.signerProvider.importPrivateKey !== "function"
                ) {
                    return {};
                }

                const signer = await state.provider.signerProvider.importPrivateKey(privateKeyHex);
                const publicKeyHex = await signer.getPublicKeyHex();

                if (registerUser) {
                    if (typeof signer.getPrivateKeyHex !== "function") {
                        console.log("signer.getPrivateKeyHex is not function");
                        return {};
                    }
                    const privateKeyHex = await signer.getPrivateKeyHex();
                    await state.provider.signerProvider.registerUser(privateKeyHex);
                }

                const iqWallets = await getAllIQWallets(state.provider);

                if (changeCurrent) {
                    const wallet = iqWallets.filter((x) => x.publicKeyHex === publicKeyHex)[0];
                    if (wallet) {
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
        setWalletProvider: (provider: QedUserWalletProvider) =>
            setAsync(async ({ set, get, state }) => {
                if (provider === state.provider) {
                    return {};
                }
                const wallets = await getAllIQWallets(provider);
                return {
                    provider,
                    signerAbilities: provider.signerProvider.getAbilities(),
                    walletAbilities: wallets[0] ? wallets[0].wallet.signer.getAbilities() : [],
                    wallets,
                    networkId: provider.networkId,
                    coordinator: provider.coordinatorEdgeRpcProvider,
                    realm: provider.realmEdgeRpcProvider,
                    currentWallet: wallets[0] || undefined,
                };
            }, true),
    };
});

export { useWalletState, WalletWidgetLoadingState };
