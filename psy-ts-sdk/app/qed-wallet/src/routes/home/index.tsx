import {
    QedWalletWidget,
    createMemoryWalletProvider,
} from "@qed/psy-wallet-widget";
import { IconSettings } from "@tabler/icons-react";
import React, { useState, useEffect } from 'react';
import { CityRollupLogoCon, SettingsButtonContainer, SettingsButton } from "./Home.styles";
import logoImage from "../../assets/psy.png";
import NetworkSettings from "../../components/NetworkSettings";
import { useWalletConfig } from "../../config";
import { TokensProvider } from "../../contexts/TokensContext";
import { QedWasmWebProverProvider, WasmRpcServer, QedJSON, initWasmSync} from "@qed/psy-sdk";

const HomePage: React.FC = () => {
    const { config } = useWalletConfig();
    const [networkSettingsOpen, setNetworkSettingsOpen] = useState(false);

    useEffect(() => {
        const initWasmRpcServer = async () => {
            try {
                const rpcConfigJson = {
                    global_user_tree_height: config.network.global_user_tree_height,
                    realm_user_tree_height: config.network.realm_user_tree_height,
                    users_per_realm: config.network.users_per_realm,
                    realm_configs: config.network.realm_configs,
                    coordinator_configs: config.network.coordinator_configs,
                    prover_url: config.network.prover_url as string,
                    prove_proxy_url: config.network.prove_proxy_url as string,
                };
                const json = QedJSON.stringify(rpcConfigJson);
                const now = new Date().getTime();
                initWasmSync();
                QedWasmWebProverProvider.wasmServer = await new WasmRpcServer(json);
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
            } catch (error) {
                console.error('Failed to get prover URL:', error);
            }
        };

        initWasmRpcServer();

        return () => {

            console.log('Component will unmount');
        };
    }, []);

    const walletProvider = createMemoryWalletProvider(
        config.network.global_user_tree_height,
        config.network.realm_user_tree_height,
        config.network.coordinator_configs,
        config.network.realm_configs,
        config.network.users_per_realm,
        config.network.prover_url as string,
        config.network.prove_proxy_url as string[],
    );

    return (
        <TokensProvider>
            <QedWalletWidget provider={walletProvider}>
                <CityRollupLogoCon>
                    <img src={logoImage} alt="Psy Wallet" />
                </CityRollupLogoCon>
                <SettingsButtonContainer>
                    <SettingsButton onClick={() => setNetworkSettingsOpen(true)}>
                        <IconSettings size={20} />
                    </SettingsButton>
                </SettingsButtonContainer>
            </QedWalletWidget>

            {/* Network Settings Modal */}
            <NetworkSettings
                opened={networkSettingsOpen}
                onClose={() => setNetworkSettingsOpen(false)}
            />
        </TokensProvider>
    );
};

export default HomePage;
