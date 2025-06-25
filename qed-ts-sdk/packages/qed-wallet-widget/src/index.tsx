export type { IQedWidgetWallet } from "./types";
export { QedWalletWidget } from "./components/QedWalletWidget";
export { useWalletState } from "./hooks/useWalletState";
export { useAddressModal, AddressModalType } from "./hooks/useAddressModal";
export type { IWalletWidgetRPC } from "./utils/rpc/types";
export * from "./utils/rpc/walletRPC";

export { createMemoryWalletProvider, createMemoryWalletProviderWithWebProver} from "./utils/provider";

// Theme exports
export { WalletThemeProvider, useTheme } from "./themes/ThemeProvider";
export { lightTheme, darkTheme, extensionTheme } from "./themes";
export type { WalletTheme } from "./themes";
