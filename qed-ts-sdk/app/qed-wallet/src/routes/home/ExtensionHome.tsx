import React, { useState, useEffect } from "react";
import {
    QedWalletWidget,
    createMemoryWalletProvider
} from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { useWalletConfig } from "../../config";
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
    const [error] = useState<string | null>(null);
    const { config } = useWalletConfig();

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

    const walletProvider = createMemoryWalletProvider(
        config.network.global_user_tree_height ?? 24,
        config.network.realm_user_tree_height ?? 23,
        config.network.coordinator_configs, // coordinator
        config.network.realm_configs, // realm
        config.network.users_per_realm,
        config.network.prover_url as string,
        config.network.prove_proxy_url as string,
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
