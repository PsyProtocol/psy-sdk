import React, { useEffect, useMemo, useState } from 'react';
import styles from './CityWalletWidget.module.scss';
import { useWalletState } from '../../hooks/useWalletState';
import { StatefulAddressSelector } from '../AddressSelector';
import { AddressModal } from '../AddressModal';
import { StatefulAddressHeader } from '../AddressHeader';
import { WalletActions } from '../WalletActions';
import { CityUserWalletProvider } from '@qstudio/city-sdk';
interface ICityWalletWidgetProps {
  className?: string;
  provider: CityUserWalletProvider;
  children?: React.ReactNode;
}
const CityWalletWidget: React.FC<ICityWalletWidgetProps> = ({ className, provider, children }) => {
  const [setWalletProvider] = useWalletState((state)=>[state.setWalletProvider]);

  useEffect(()=>{
    setWalletProvider(provider);
  },[provider]);
  return (
    <div className={styles.walletWidget + (className?(" "+className):"")}>
      <div className={styles.walletWidgetHeader}>
        <StatefulAddressSelector />
      </div>
      <div className={styles.walletWidgetBody}>
        <StatefulAddressHeader />
        <WalletActions />
        {children}
      </div>
      <AddressModal />
    </div>
  );
}

export {
  CityWalletWidget,
}