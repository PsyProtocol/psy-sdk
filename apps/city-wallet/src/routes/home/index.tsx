import React from 'react';
import { CityWalletWidget, createMemoryWalletProvider } from '@qstudio/city-wallet-widget';
import logoImage from '../../assets/city-rollup-logo.png';
import styles from './Home.module.scss';

const HomePage: React.FC = () => {
  const walletProvider = createMemoryWalletProvider("http://localhost:3000?networkId=dogeRegtest", "http://localhost:1447");

  return (
    <CityWalletWidget provider={walletProvider}>
      <div className={styles.cityRollupLogoCon}>
        <img src={logoImage} alt="City Rollup Wallet" className={styles.walletLogo} />
      </div>
    </CityWalletWidget>
  );
};

export default HomePage;