import React, { useEffect, useMemo } from 'react';
import styles from './WalletWidget.module.scss';
import { useWalletState } from '../../hooks/useWalletState';
import { StatefulAddressSelector } from '../AddressSelector';
import { WidgetDogeWalletProvider } from '../../utils/provider';
import { AddressModal } from '../AddressModal';
import { AddressHeader, StatefulAddressHeader } from '../AddressHeader';
import { WalletActions } from '../WalletActions';
import { WalletTransactions } from '../Transactions';
import {SeedRandom, hexToU8Array} from "@qstudio/utils";
import { encodePrivateKeyToWIF } from 'doge-sdk/dist/types';
interface IWalletWidgetProps {
  className?: string;
  provider: WidgetDogeWalletProvider<any>;
}
const WalletWidget: React.FC<IWalletWidgetProps> = ({ className, provider }) => {
  const [setWalletProvider] = useWalletState((state)=>[state.setWalletProvider]);

  useEffect(()=>{
    setWalletProvider(provider)
  },[provider]);
  return (
    <div className={styles.walletWidget + (className?(" "+className):"")}>
      <div className={styles.walletWidgetHeader}>
        <StatefulAddressSelector />
      </div>
      <div className={styles.walletWidgetBody}>
        <StatefulAddressHeader />
        <WalletActions />
        <div className={styles.wwTransactionCon}>
        <WalletTransactions />
        </div>
      </div>
      <AddressModal />
    </div>
  );
}

export {
  WalletWidget,
}