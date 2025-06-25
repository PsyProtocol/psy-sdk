import React, { useState } from 'react';
import { useWalletState, useAddressModal, AddressModalType, IQedWidgetWallet } from "@qed/qed-wallet-widget";
import logoImage from "../../assets/psy.png";
import { useWalletConfig } from "../../config";
import { IconSettings } from "@tabler/icons-react";
import { useNavigate } from "react-router-dom";
import WalletSelector from "../../components/WalletSelector";
import Banner from "../../components/Banner";
import ActionButtons from "../../components/ActionButtons";
import BottomNavigation from "../../components/BottomNavigation";
import TokensList from "../../components/TokensList";
import WalletBalance from "../../components/WalletBalance";
import TransactModal, { TransactType } from "../../components/TransactModal";
import NetworkSettings from "../../components/NetworkSettings";
import { usePersistentWallet } from "../../hooks/usePersistentWallet";
import WalletOnboarding from "../../components/WalletOnboarding";
import {
    Header,
    HeaderLeft,
    HeaderRight,
    MainContent,
    SettingsButton
} from "./ExtensionHome.styles";
import { useBlockNumber } from 'packages/qed-wallet-widget/src/utils/data';
import { CheckPoint } from '../../components/WalletSelector/WalletSelector.styles';

export const ExtensionContent: React.FC = () => {
    const [activeTab, setActiveTab] = useState<'home' | 'tokens'>('home');
    const [transactModal, setTransactModal] = useState<{
        opened: boolean;
        type: TransactType | null;
    }>({ opened: false, type: null });
    const [networkSettingsOpen, setNetworkSettingsOpen] = useState(false);
    const [isCheckingWallet, setIsCheckingWallet] = useState(true);
    const { config } = useWalletConfig();
    const navigate = useNavigate();

    const [wallets, currentWallet, addRandomWallet, refreshAllWallets, setActiveWalletAsync, providerAbilities, provider] = useWalletState(
        (state) => [
            state.wallets,
            state.currentWallet,
            state.addRandomWallet,
            state.refreshAllWallets,
            state.setActiveWalletAsync,
            state.providerAbilities,
            state.provider
        ]
    );
    const [openModal, modalState] = useAddressModal(state => [state.openModal, state]);
    // Initialize persistent wallet storage
    const { clearStoredWallets } = usePersistentWallet();

    // Check wallet status after a short delay to allow wallet state to initialize
    React.useEffect(() => {
        const checkWalletTimer = setTimeout(() => {
            console.log('Wallet initialization check - wallets count:', wallets.length);
            setIsCheckingWallet(false);
        }, 1500); // Give more time for wallet state and restoration to complete

        return () => clearTimeout(checkWalletTimer);
    }, []);


    // Check if we have any wallets available
    const hasWallets = wallets.length > 0;

    // Debug logging and wallet restoration detection
    React.useEffect(() => {
        console.log('Wallet state:', {
            walletsCount: wallets.length,
            hasCurrentWallet: !!currentWallet,
            currentWalletAddress: currentWallet?.address,
            isCheckingWallet,
            hasWallets
        });

        // If wallets are restored while still checking, stop checking immediately
        if (isCheckingWallet && wallets.length > 0) {
            console.log('Wallets restored, stopping check');
            setIsCheckingWallet(false);
        }
    }, [wallets.length, currentWallet?.address, isCheckingWallet, hasWallets]);

    const handleNewWallet = async () => {
        if (providerAbilities.includes("add-random-private-key")) {
            await addRandomWallet(true);
        }
    };

    const handleImportWallet = () => {
        openModal(AddressModalType.Import, undefined, {
            onComplete: async (resultData) => {
                console.log('Import completed:', resultData);
                // Immediately refresh all wallets to trigger auto-selection
                await refreshAllWallets();
                // Also clear the checking state to ensure we show the main interface
                setIsCheckingWallet(false);
            }
        });
    };

    const handleSelectWallet = async (userId: number) => {
        try {
            await setActiveWalletAsync(userId);
            await refreshAllWallets();
            console.log('Wallet selected and refreshed successfully');
        } catch (error) {
            console.error('Error selecting wallet:', error);
        }
    };

    const handleRefreshWallets = async () => {
        await refreshAllWallets();
    };

    const handleTransfer = () => {
        setTransactModal({ opened: true, type: 'transfer' });
    };

    const handleMint = () => {
        setTransactModal({ opened: true, type: 'mint' });
    };

    const handleClaim = () => {
        setTransactModal({ opened: true, type: 'claim' });
    };

    const handleTransactClose = () => {
        setTransactModal({ opened: false, type: null });
    };

    const handleTransactConfirm = (data: any) => {
        console.log(`${transactModal.type} transaction completed:`, data);
    };

    const checkpointId = useBlockNumber(provider, 1000);

    // Show loading state while checking wallets
    if (isCheckingWallet) {
        return (
            <div style={{
                height: '100%',
                width: '100%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                backgroundColor: config.theme.colors.background,
                position: 'relative',
                zIndex: 10
            }}>
                <div style={{ textAlign: 'center', color: config.theme.colors.text }}>
                    <img src={logoImage} alt="Loading" style={{ width: 48, marginBottom: 16 }} />
                    <div>Initializing wallet...</div>
                </div>
            </div>
        );
    }

    // Show onboarding if no wallets exist
    if (!hasWallets) {
        return (
            <div style={{
                height: '100%',
                width: '100%',
                backgroundColor: config.theme.colors.background,
                position: 'relative',
                zIndex: 10
            }}>
                <WalletOnboarding
                    onCreateWallet={handleNewWallet}
                    onImportWallet={handleImportWallet}
                />
            </div>
        );
    }

    // Show main wallet interface
    return (
        <div style={{
            height: '100%',
            width: '100%',
            display: 'flex',
            flexDirection: 'column',
            backgroundColor: config.theme.colors.background,
            position: 'relative',
            zIndex: 10
        }}>
            <Header>
                <HeaderLeft>
                    <WalletSelector
                        wallets={wallets}
                        currentWallet={currentWallet ? {
                            name: currentWallet.name,
                            address: currentWallet.address
                        } : undefined}
                        onNewWallet={handleNewWallet}
                        onImportWallet={handleImportWallet}
                        onRefreshWallets={handleRefreshWallets}
                        onSelectWallet={handleSelectWallet}
                    />
                </HeaderLeft>
                <CheckPoint>
                    Checkpoint: {checkpointId}
                </CheckPoint>
                <HeaderRight>
                    <SettingsButton onClick={() => setNetworkSettingsOpen(true)}>
                        <IconSettings size={20} />
                    </SettingsButton>
                </HeaderRight>
            </Header>

            <MainContent>
                {activeTab === 'home' ? (
                    <>
                        <WalletBalance />
                        <Banner />
                        <ActionButtons
                            onTransfer={handleTransfer}
                            onMint={handleMint}
                            onClaim={handleClaim}
                        />
                    </>
                ) : (
                    <TokensList />
                )}
            </MainContent>

            <BottomNavigation
                activeTab={activeTab}
                onTabChange={setActiveTab}
            />

            {/* Transact Modal */}
            {transactModal.type && (
                <TransactModal
                    opened={transactModal.opened}
                    onClose={handleTransactClose}
                    type={transactModal.type}
                    onConfirm={handleTransactConfirm}
                />
            )}

            {/* Network Settings Modal */}
            <NetworkSettings
                opened={networkSettingsOpen}
                onClose={() => setNetworkSettingsOpen(false)}
            />
        </div>
    );
};

export default ExtensionContent;