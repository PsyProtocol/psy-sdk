import React from 'react';
import { useWalletState } from "@qed/qed-wallet-widget";
import { useWalletConfig } from '../../config';
import { BalanceContainer, BalanceAmount, BalanceCurrency } from './WalletBalance.styles';
import { useBlockNumber, useUserBalance } from 'packages/qed-wallet-widget/src/utils/data';
import { useTokens } from '../../contexts/TokensContext';

export const WalletBalance: React.FC = () => {
  const { getNativeCurrency } = useWalletConfig();
  const [currentWallet, refreshCurrentWallet, walletProvider] = useWalletState((state) => [
    state.currentWallet,
    state.refreshCurrentWallet,
    state.provider,
  ]);


  // Format balance to display with proper decimals
  const formatBalance = (balance: number | undefined): string => {
    if (balance === undefined || balance === null) return '0.000';
    
    // Convert from wei-like units (10^9) to display units
    const divisor = 1000000000; // 10^9
    const formattedAmount = balance / divisor;
    return formattedAmount.toFixed(3);
  };

  const contractId = parseInt(getNativeCurrency(), 10);
  const userId = !currentWallet ? 0 : currentWallet.userId;
  const checkpointId = useBlockNumber(walletProvider, 1000);
  const balance = useUserBalance(walletProvider, checkpointId, userId, contractId, 1000);

  const { tokens } = useTokens();
  const currency = tokens[contractId]?.symbol || "PSY";

  return (
    <BalanceContainer>
      <BalanceAmount>{formatBalance(Number(balance))}</BalanceAmount>
      <BalanceCurrency>{currency}</BalanceCurrency>
    </BalanceContainer>
  );
};

export default WalletBalance;