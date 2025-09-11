import { StoreApi, create } from "zustand";
import { IQedWidgetWallet, DEFAULT_WALLET_NAME } from "../types";
import {
    TQedTransactionSignerAbility,
    TQedTransactionSignerProviderAbility,
    ICoordinatorEdgeRpcProvider,
    IRealmEdgeRpcProvider,
    IQedUserWallet,
} from "@qed/qed-sdk";
import { createMemoryWalletProvider } from "../utils/provider";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";
import { loadConfig } from "../config";

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
    hasTriedRestore: boolean;
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
                name: user.status ? userInfo.userId.toString() : DEFAULT_WALLET_NAME,
                address: userInfo.publicKeyHex,
                wallet: user,
                isActive: false,
            } as IQedWidgetWallet;
        })
    );
}
function setAsyncFactory(set: Setter, get: Getter, _api: WidgetStoreAPI) {
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
                console.log(err);
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

const useWalletState = create<IWalletStateStore>((set, get, api) => {
    const config = loadConfig();
    const setAsync = setAsyncFactory(set, get, api);
    const walletProvider = createMemoryWalletProvider(
        config.network.global_user_tree_height,
        config.network.realm_user_tree_height,
        config.network.coordinator_configs, // coordinator
        config.network.realm_configs, // realm
        config.network.users_per_realm,
        config.network.prover_url, // prover
        config.network.prove_proxy_url,
    );

    return {
        loadingState: WalletWidgetLoadingState.Ready,
        provider: walletProvider,
        wallets: [],
        providerAbilities: walletProvider.signerProvider.getAbilities(),
        walletAbilities: [],
        hasTriedRestore: false,
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
            set((_state) => {
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
                if (!wallet.wallet.status) {
                    return {};
                }
                const userInfo = await wallet.wallet.getUserInfo();

                const wallets = state.wallets.map((w) => {
                    if (w.wallet.statue && w.userId === wallet.userId) {
                        return { ...userInfo, name: userInfo.userId.toString(), address: userInfo.publicKeyHex, wallet: wallet.wallet };
                    } else {
                        return w;
                    }
                });

                // Sync the RPC provider with the new active wallet
                state.provider.realmEdgeRpcProvider.setUserId(userId);

                return {
                    wallets,
                    currentWallet: { ...userInfo, name: wallet.wallet ? userInfo.userId.toString() : DEFAULT_WALLET_NAME, address: userInfo.publicKeyHex, wallet: wallet.wallet },
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
                if (!wallet.wallet.status) {
                    return {};
                }
                // Sync the RPC provider with the new active wallet
                state.provider.realmEdgeRpcProvider.setUserId(userId);
                return { currentWallet: wallet };
            }),

        addRandomWallet: (registerUser?: boolean) =>
            setAsync(async ({ set: _set, get: _get, state }) => {
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
                    await state.provider.signerProvider.addUser(privateKeyHex);
                }

                // Refresh wallet list and set new wallet as current
                const iqWallets = await getAllIQWallets(state.provider);
                const newWallet = iqWallets.filter((x) => x.publicKeyHex === publicKeyHex)[0];

                if (newWallet && newWallet.wallet.status) {
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
            setAsync(async ({ set: _set, get: _get, state }) => {
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
                    await state.provider.signerProvider.addUser(privateKeyHex);
                }

                console.log("Getting all IQ wallets...");
                const iqWallets = await getAllIQWallets(state.provider);
                console.log("Found", iqWallets.length, "wallets:", iqWallets.map(w => ({ userId: w.userId, address: w.address })));

                if (changeCurrent) {
                    const wallet = iqWallets.filter((x) => x.publicKeyHex === publicKeyHex)[0];
                    console.log("Found matching wallet for current:", wallet);
                    if (wallet && wallet.wallet.status) {
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
            setAsync(async ({ set: _set, get: _get, state }) => {
                if (provider === state.provider) {
                    return {};
                }

                if (!state.hasTriedRestore) {
                    try {
                        const WALLET_STORAGE_KEY = 'psy_wallet_data';
                        const stored = localStorage.getItem(WALLET_STORAGE_KEY);

                        if (stored) {
                            const data = JSON.parse(stored);
                            const isDataFresh = Date.now() - data.lastUpdated < 24 * 60 * 60 * 1000;

                            if (isDataFresh && data.wallets.length > 0) {
                                console.log('Restoring first wallet from storage...');
                                const firstWallet = data.wallets[0];
                                if (firstWallet?.privateKey) {
                                    await provider.signerProvider.importPrivateKey(firstWallet.privateKey);
                                }
                            }
                        }
                    } catch (error) {
                        console.warn('Failed to restore wallets:', error);
                    }
                }

                const wallets = await getAllIQWallets(provider);
                return {
                    provider,
                    providerAbilities: provider.signerProvider.getAbilities(),
                    walletAbilities: wallets[0] ? wallets[0].wallet.signer.getAbilities() : [],
                    wallets,
                    hasTriedRestore: true,
                    networkId: provider.networkId,
                    coordinatorEdgeRpcProvider: provider.coordinatorEdgeRpcProvider,
                    realmEdgeRpcProvider: provider.realmEdgeRpcProvider,
                    currentWallet: wallets[0] || undefined,
                };
            }, true),
    };
});

export { useWalletState, WalletWidgetLoadingState };
