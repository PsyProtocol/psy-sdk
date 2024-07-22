import { ActionIcon, Button, CopyButton, Tooltip, rem } from '@mantine/core';
import {IconCopy, IconCheck, IconRefresh }from '@tabler/icons-react';

import styles from './AddressHeader.module.scss';
import React, { useState } from 'react';
import { WWCopyButton } from '../WWCopyButton';
import { useWalletState } from '../../hooks/useWalletState';
import { formatBalance } from '../../utils/balance';
interface IAddressHeaderProps {
  address: string;
  balance: string;
  onRefresh?: ()=>Promise<void>;
}



const AddressHeader: React.FC<IAddressHeaderProps> = ({ address, balance, onRefresh }) => {
  const [loading, setLoading] = useState(false);
  return (
    <div className={styles.addressHeader}>
    <div className={styles.addressHeaderItem}>
      <div className={styles.addressHint}>Wallet Address</div>
      <div className={styles.addressValue}>
        <span className={styles.address}>{address}</span>
        <WWCopyButton value={address} />
        </div>
    </div>
        <div className={styles.addressHeaderItem}>
          <div className={styles.addressHint}>Balance</div>
          <div className={styles.addressValue}>
            <span className={styles.innerValue}>{balance}</span>
            {onRefresh?<ActionIcon variant="subtle" color="gray" loading={loading} onClick={()=>{
              setLoading(true);
              onRefresh().then(()=>setLoading(false)).catch(()=>setLoading(false));
            }}><IconRefresh style={{ width: rem(16) }}/></ActionIcon>:null}
            </div>
        </div>
    </div>
  );
}

const StatefulAddressHeader: React.FC = () => {
  const [currency, currentWallet, refreshCurrentWallet] = useWalletState((state)=>[state.currency, state.currentWallet,state.refreshCurrentWallet]);
  if(!currentWallet){
    return <div className={styles.noWalletAddressHeader}>Please select a wallet above or import a wallet to get started.</div>;
  }

  return (
    <AddressHeader address={currentWallet.userId+""} balance={formatBalance(currentWallet.balance, currency)} onRefresh={()=>refreshCurrentWallet()} />
  );
}



export {
  AddressHeader,
  StatefulAddressHeader,
}