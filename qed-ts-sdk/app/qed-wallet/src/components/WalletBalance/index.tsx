import React from 'react';
import { useWalletState } from "@qed/qed-wallet-widget";
import { useWalletConfig } from '../../config';
import { BalanceContainer, BalanceAmount, BalanceCurrency } from './WalletBalance.styles';

export const WalletBalance: React.FC = () => {
  const { config } = useWalletConfig();
  const [currentWallet] = useWalletState((state) => [state.currentWallet]);

  // Format balance to display
  const formatBalance = (balance: number | undefined): string => {
    if (balance === undefined || balance === null) return '0.00';
    return balance.toFixed(2);
  };

  const balance = currentWallet?.balance || 0;

  return (
    <BalanceContainer>
      <BalanceAmount>{formatBalance(balance)}</BalanceAmount>
      <BalanceCurrency>PSY</BalanceCurrency>
    </BalanceContainer>
  );
};

export default WalletBalance;