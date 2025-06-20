import React from "react";
import { QedWalletWidget, createMemoryWalletProvider } from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import styles from "./Home.module.scss";

const HomePage: React.FC = () => {
    const walletProvider = createMemoryWalletProvider(
        "http://localhost:8545",
        "http://localhost:8546",
        "http://localhost:8888",
    );

    return (
        <QedWalletWidget provider={walletProvider}>
            <div className={styles.cityRollupLogoCon}>
                <img src={logoImage} alt="Psy Wallet" className={styles.walletLogo} />
            </div>
        </QedWalletWidget>
    );
};

export default HomePage;
