import React, { useEffect, useMemo, useState } from "react";
import { useWalletState } from "../../hooks/useWalletState";
import { StatefulAddressSelector } from "../AddressSelector";
import { AddressModal } from "../AddressModal";
import { StatefulAddressHeader } from "../AddressHeader";
import { WalletActions } from "../WalletActions";
import { PsyUserWalletProvider } from "@psy/psy-sdk/src/wallet/provider";
import { WalletThemeProvider } from "../../themes/ThemeProvider";
import { GlobalStyles } from "../../themes/GlobalStyles";
import {
    WalletWidgetContainer,
    WalletWidgetHeader,
    WalletWidgetBody
} from "./PsyWalletWidget.styles";

interface IPsyWalletWidgetProps {
    className?: string;
    provider: PsyUserWalletProvider;
    children?: React.ReactNode;
    theme?: 'light' | 'dark' | 'extension';
}

const PsyWalletWidgetInner: React.FC<Omit<IPsyWalletWidgetProps, 'theme'>> = ({ 
    className, 
    provider, 
    children 
}) => {
    const [setWalletProvider] = useWalletState((state) => [state.setWalletProvider]);

    useEffect(() => {
        setWalletProvider(provider);
    }, [provider]);
    
    return (
        <>
            <GlobalStyles />
            <WalletWidgetContainer className={className}>
                <WalletWidgetHeader>
                    <StatefulAddressSelector />
                </WalletWidgetHeader>
                <WalletWidgetBody>
                    <StatefulAddressHeader />
                    <WalletActions />
                    {children}
                </WalletWidgetBody>
                <AddressModal />
            </WalletWidgetContainer>
        </>
    );
};

const PsyWalletWidget: React.FC<IPsyWalletWidgetProps> = ({ 
    theme = 'dark',
    ...props 
}) => {
    // Detect if running in extension
    const isExtension = typeof window !== 'undefined' && window.location.protocol === 'chrome-extension:';
    const defaultTheme = isExtension ? 'extension' : theme;
    
    return (
        <WalletThemeProvider defaultTheme={defaultTheme}>
            <PsyWalletWidgetInner {...props} />
        </WalletThemeProvider>
    );
};

export { PsyWalletWidget };
