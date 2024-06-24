import { useEffect, useMemo, useRef, useState } from 'react';
import styles from './Wallet.module.scss';
import { IDEContext } from '../../utils/ideContext';

interface IWalletDockComponentProps {
  ctx: IDEContext;
}
function waitMs(ms: number) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}
const WalletDockComponent: React.FC<IWalletDockComponentProps> = ({ ctx }) => {
  const onRun = async () => {
    console.log("running");

    

    





  };
  return (
    <div className={styles.walletDockPage}>
      <div className={styles.walletContent}>
        <button onClick={onRun}>Run</button>

      </div>
    </div>
  )
};

export default WalletDockComponent;