import React, { useState, useEffect } from "react";
import {
    QedWalletWidget,
    // createMemoryWalletProvider,
    createMemoryWalletProviderWithWebProver
} from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import {defaultConfig, useWalletConfig} from "../../config";
import { TokensProvider } from "../../contexts/TokensContext";
import ExtensionContent from "./ExtensionContent";
import {
    ExtensionContainer,
    LoadingContainer,
    LoadingContent,
    Logo,
    ErrorContainer,
    ErrorTitle,
    ErrorMessage,
    ErrorHint
} from "./ExtensionHome.styles";

const ExtensionHomeContent: React.FC = () => {
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const { config, getCoordinatorUrl, getRealmUrl, getProverUrl } = useWalletConfig();

    useEffect(() => {
        // Check if we're in extension context
        if (window.location.protocol === 'chrome-extension:') {
            console.log('Running as Chrome extension');
        }

        // Quick loading
        setTimeout(() => {
            setIsLoading(false);
        }, 10);
    }, []);

    // const walletProvider = createMemoryWalletProvider(
    //     getCoordinatorUrl(), // coordinator
    //     getRealmUrl(), // realm
    //     getProverUrl(), // prover
    // );

    const walletProvider = createMemoryWalletProviderWithWebProver(
        getCoordinatorUrl(), // coordinator
        getRealmUrl(), // realm
        {
            users_per_realm: defaultConfig.network.users_per_realm,
            realm_configs:defaultConfig.network.realm_configs,
            coordinator_configs: defaultConfig.network.coordinator_configs,
        }
    );

    if (isLoading) {
        return (
            <LoadingContainer>
                <LoadingContent>
                    <Logo src={logoImage} alt="QED" />
                    <div>Loading Psy Wallet...</div>
                </LoadingContent>
            </LoadingContainer>
        );
    }

    if (error) {
        return (
            <ErrorContainer>
                <div>
                    <ErrorTitle>Connection Error</ErrorTitle>
                    <ErrorMessage>{error}</ErrorMessage>
                    <ErrorHint>
                        Make sure your local QED node is running on port 8545
                    </ErrorHint>
                </div>
            </ErrorContainer>
        );
    }

    return (
        <ExtensionContainer>
            <QedWalletWidget provider={walletProvider} theme="extension">
                <TokensProvider>
                    <ExtensionContent />
                </TokensProvider>
            </QedWalletWidget>
        </ExtensionContainer>
    );
};

const ExtensionHomePage: React.FC = () => {
    return <ExtensionHomeContent />;
};

export default ExtensionHomePage;
