import {
    QedWalletWidget,
    createMemoryWalletProvider,
} from "@qed/qed-wallet-widget";
import { IconSettings } from "@tabler/icons-react";
import React, { useState } from 'react';
import { CityRollupLogoCon, SettingsButtonContainer, SettingsButton } from "./Home.styles";
import logoImage from "../../assets/psy.png";
import NetworkSettings from "../../components/NetworkSettings";
import { useWalletConfig } from "../../config";
import { TokensProvider } from "../../contexts/TokensContext";

const HomePage: React.FC = () => {
    const { config, getProverUrl } = useWalletConfig();
    const [networkSettingsOpen, setNetworkSettingsOpen] = useState(false);

    const walletProvider = createMemoryWalletProvider(
        config.network.global_user_tree_height,
        config.network.realm_user_tree_height,
        config.network.coordinator_configs,
        config.network.realm_configs,
        config.network.users_per_realm,
        config.network.prover_url as string,
        config.network.prove_proxy_url as string,
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
