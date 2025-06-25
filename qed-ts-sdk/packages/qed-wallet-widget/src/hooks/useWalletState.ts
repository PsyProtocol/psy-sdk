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
    console.log("getAllIQWallets: Getting user wallets from provider...");
    const users = await provider.getUserWallets();
    console.log("getAllIQWallets: Found", users.length, "user wallets");

    return Promise.all(
        users.map(async (user: IQedUserWallet, index) => {
            console.log(`getAllIQWallets: Getting user info for wallet ${index}...`);
            const userInfo = await user.getUserInfo();
            console.log(`getAllIQWallets: Wallet ${index} info:`, userInfo);
            return {
                ...userInfo,
                name: userInfo.userId.toString(),
                address: userInfo.publicKeyHex,
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
    const newwork_config = {
        users_per_realm: 32768,
        realm_configs: [
            {
                id: 0,
                rpc_url: ["http://127.0.0.1:8546"],
            },
            {
                id: 16384,
                rpc_url: ["http://127.0.0.1:8547"],
            },
            {
                id: 8192,
                rpc_url: ["http://127.0.0.1:8548"],
            },
        ],
        coordinator_configs: [
            {
                id: 0,
                rpc_url: ["http://127.0.0.1:8545"],
            },
        ],
        prover_url: "http://127.0.0.1:8888",
        nativeCurrency: "0",
    };
    const setAsync = setAsyncFactory(set, get, api);
    const walletProvider = createMemoryWalletProvider(
        newwork_config.coordinator_configs, // coordinator
        newwork_config.realm_configs, // realm 
        newwork_config.users_per_realm,
        newwork_config.prover_url, // prover
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
                    state.provider.realmEdgeRpcProvider.setUserId(wallets[0].userId);
                    return {
                        wallets: wallets,
                        currentWallet: wallets[0],
                        walletAbilities: wallets[0] ? wallets[0].wallet.signer.getAbilities() : [],
                    };
                } else {
                    const currentWalletId = state.currentWallet?.userId;
                    const currentWallet = wallets.find((w) => w.userId === currentWalletId);
                    if (currentWallet) {
                        state.provider.realmEdgeRpcProvider.setUserId(currentWallet.userId);
                        return {
                            wallets,
                            currentWallet: currentWallet,
                            walletAbilities: currentWallet.wallet.signer.getAbilities(),
                        };
                    } else {
                        state.provider.realmEdgeRpcProvider.setUserId(wallets[0].userId);
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
                const publicKeyHex = await signer.getPublicKeyHex();

                if (registerUser) {
                    if (typeof signer.getPrivateKeyHex !== "function") {
                        return {};
                    }
                    const privateKeyHex = await signer.getPrivateKeyHex();
                    await state.provider.signerProvider.registerUser(privateKeyHex);
                }

                // Refresh wallet list and set new wallet as current
                const iqWallets = await getAllIQWallets(state.provider);
                const newWallet = iqWallets.filter((x) => x.publicKeyHex === publicKeyHex)[0];

                if (newWallet) {
                    return {
                        wallets: iqWallets,
                        currentWallet: newWallet,
                        walletAbilities: newWallet.wallet.signer.getAbilities(),
                    };
                }

                return {
                    wallets: iqWallets,
                };
            }),

        addWalletFromPrivateKey: (privateKeyHex: string, registerUser?: boolean, changeCurrent?: boolean) =>
            setAsync(async ({ set, get, state }) => {
                console.log("addWalletFromPrivateKey called with:", { privateKeyHex, registerUser, changeCurrent });

                if (
                    !state.providerAbilities.includes("import-private-key") ||
                    typeof state.provider.signerProvider.importPrivateKey !== "function"
                ) {
                    console.log("Provider does not support import-private-key");
                    return {};
                }

                console.log("Calling importPrivateKey on provider...");
                const signer = await state.provider.signerProvider.importPrivateKey(privateKeyHex);
                const publicKeyHex = await signer.getPublicKeyHex();
                console.log("Created signer with public key:", publicKeyHex);

                if (registerUser) {
                    if (typeof signer.getPrivateKeyHex !== "function") {
                        console.log("signer.getPrivateKeyHex is not function");
                        return {};
                    }
                    const privateKeyHex = await signer.getPrivateKeyHex();
                    console.log("Registering user with provider...");
                    await state.provider.signerProvider.registerUser(privateKeyHex);
                }

                console.log("Getting all IQ wallets...");
                const iqWallets = await getAllIQWallets(state.provider);
                console.log("Found", iqWallets.length, "wallets:", iqWallets.map(w => ({ userId: w.userId, address: w.address })));

                if (changeCurrent) {
                    const wallet = iqWallets.filter((x) => x.publicKeyHex === publicKeyHex)[0];
                    console.log("Found matching wallet for current:", wallet);
                    if (wallet) {
                        return {
                            wallets: iqWallets,
                            currentWallet: wallet,
                            walletAbilities: wallet.wallet.signer.getAbilities(),
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
