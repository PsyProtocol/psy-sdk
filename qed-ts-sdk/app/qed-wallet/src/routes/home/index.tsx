import React from "react";
import {
    QedWalletWidget,
    createMemoryWalletProvider,
} from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { CityRollupLogoCon } from "./Home.styles";
import { useWalletConfig } from "../../config";

const HomePage: React.FC = () => {
    const { config, getProverUrl } = useWalletConfig();

    const walletProvider = createMemoryWalletProvider(
        config.network.coordinator_configs,
        config.network.realm_configs,
        config.network.users_per_realm,
        getProverUrl(),
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
