import React from "react";
import {
    QedWalletWidget,
    // createMemoryWalletProvider,
    createMemoryWalletProviderWithWebProver
} from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { CityRollupLogoCon } from "./Home.styles";

const HomePage: React.FC = () => {
    // const walletProvider = createMemoryWalletProvider(
    //     "http://localhost:8545",
    //     "http://localhost:8546",
    //     "http://localhost:8888",
    // );

    const walletProvider = createMemoryWalletProviderWithWebProver(
        "http://localhost:8545",
        "http://localhost:8546",
        {
            users_per_realm: 32768,
            realm_configs: [
                {
                    id: 0,
                    rpc_url: ["http://127.0.0.1:8546"]
                },
                {
                    id: 16384,
                    rpc_url: ["http://127.0.0.1:8547"]
                },
                {
                    id: 8192,
                    rpc_url: ["http://127.0.0.1:8548"]
                }
            ],
            coordinator_configs: [
                {
                    id: 0,
                    rpc_url: ["http://127.0.0.1:8545"]
                }
            ],
        }
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
