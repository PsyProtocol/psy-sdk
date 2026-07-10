/**
 * React bindings for @psy-protocol/evm-wallet.
 *
 * The ONLY module in this package that imports react (optional peer). Hooks
 * mirror the mode-a app's UnifiedSessionContext / useUnifiedActivityFeed value
 * shapes so existing consumers migrate mechanically. Implemented in phase P6:
 * PsyWalletProvider, usePsyWallet, useUpsSession, useActivity, useInbox,
 * useTxStatus.
 */

export const EVM_WALLET_REACT = true;
