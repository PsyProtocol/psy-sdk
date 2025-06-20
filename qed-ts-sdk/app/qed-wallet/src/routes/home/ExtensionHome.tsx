import React, { useState, useEffect } from "react";
import { QedWalletWidget, createMemoryWalletProvider } from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { useWalletConfig } from "../../config";
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

const ExtensionHomePage: React.FC = () => {
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
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
        "http://localhost:8545", // coordinator
        "http://localhost:8546", // realm 
        "http://localhost:8888", // prover
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
                <ExtensionContent />
            </QedWalletWidget>
        </ExtensionContainer>
    );
};

export default ExtensionHomePage;
