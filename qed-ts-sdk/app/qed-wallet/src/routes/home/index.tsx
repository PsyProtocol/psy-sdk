import React from "react";
import { QedWalletWidget, createMemoryWalletProvider } from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { CityRollupLogoCon } from "./Home.styles";

const HomePage: React.FC = () => {
    const walletProvider = createMemoryWalletProvider(
        "http://localhost:8545",
        "http://localhost:8546",
        "http://localhost:8888",
    );

    return (
        <QedWalletWidget provider={walletProvider}>
            <CityRollupLogoCon>
                <img src={logoImage} alt="Psy Wallet" />
            </CityRollupLogoCon>
        </QedWalletWidget>
    );
};

export default HomePage;
