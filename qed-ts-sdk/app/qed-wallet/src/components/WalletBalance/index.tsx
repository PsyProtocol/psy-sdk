import React from 'react';
import { useWalletState } from "@qed/qed-wallet-widget";
import { useWalletConfig } from '../../config';
import { BalanceContainer, BalanceAmount, BalanceCurrency } from './WalletBalance.styles';
import { useBlockNumber, useUserBalance } from 'packages/qed-wallet-widget/src/utils/data';

export const WalletBalance: React.FC = () => {
  const { getNativeCurrency } = useWalletConfig();
  const [currency, currentWallet, refreshCurrentWallet, walletProvider] = useWalletState((state) => [
    state.currency,
    state.currentWallet,
    state.refreshCurrentWallet,
    state.provider,
  ]);


  // Format balance to display
  const formatBalance = (balance: number | undefined): string => {
    if (balance === undefined || balance === null) return '0.00';
    return balance.toFixed(2);
  };

  const contractId = parseInt(getNativeCurrency(), 10);
  const userId = !currentWallet ? 0 : currentWallet.userId;
  const checkpointId = useBlockNumber(walletProvider, 1000);
  const balance = useUserBalance(walletProvider, checkpointId, userId, contractId, 1000);


  return (
    <BalanceContainer>
      <BalanceAmount>{formatBalance(Number(balance))}</BalanceAmount>
      <BalanceCurrency>{currency}</BalanceCurrency>
    </BalanceContainer>
  );
};

export default WalletBalance;