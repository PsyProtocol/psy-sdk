export type { IQedWidgetWallet } from "./types";
export { QedWalletWidget } from "./components/QedWalletWidget";
export { useWalletState } from "./hooks/useWalletState";
export type { IWalletWidgetRPC } from "./utils/rpc/types";
export * from "./utils/rpc/walletRPC";

export { createMemoryWalletProvider } from "./utils/provider";
