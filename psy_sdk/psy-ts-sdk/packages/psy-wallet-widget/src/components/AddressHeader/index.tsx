import { ActionIcon, Button, CopyButton, Tooltip, rem } from "@mantine/core";
import { IconCopy, IconCheck, IconRefresh } from "@tabler/icons-react";

import React, { useState } from "react";
import { WWCopyButton } from "../WWCopyButton";
import { useWalletState } from "../../hooks/useWalletState";
import { formatBalance } from "../../utils/balance";
import {
    AddressHeaderContainer,
    AddressHeaderItem,
    AddressHint,
    AddressValue,
    InnerValue,
    NoWalletAddressHeader
} from "./AddressHeader.styles";
import { useWalletConfig } from "../../config";
import { useBlockNumber, useUserBalance } from "../../utils/data";
interface IAddressHeaderProps {
    address: string;
    balance: string;
    onRefresh?: () => Promise<void>;
}

const AddressHeader: React.FC<IAddressHeaderProps> = ({ address, balance, onRefresh }) => {
    const [loading, setLoading] = useState(false);
    return (
        <AddressHeaderContainer>
            <AddressHeaderItem>
                <AddressHint>Wallet Address</AddressHint>
                <AddressValue>
                    <span>{address}</span>
                    <WWCopyButton value={address} />
                </AddressValue>
            </AddressHeaderItem>
            <AddressHeaderItem>
                <AddressHint>Balance</AddressHint>
                <AddressValue>
                    <InnerValue>{balance}</InnerValue>
                    {onRefresh ? (
                        <ActionIcon
                            variant="subtle"
                            color="gray"
                            loading={loading}
                            onClick={() => {
                                setLoading(true);
                                onRefresh()
                                    .then(() => setLoading(false))
                                    .catch(() => setLoading(false));
                            }}
                        >
                            <IconRefresh style={{ width: rem(16) }} />
                        </ActionIcon>
                    ) : null}
                </AddressValue>
            </AddressHeaderItem>
        </AddressHeaderContainer>
    );
};

const StatefulAddressHeader: React.FC = () => {
    const [currentWallet, refreshCurrentWallet] = useWalletState((state) => [
        state.currentWallet,
        state.refreshCurrentWallet,
    ]);
    
    if (!currentWallet) {
        return (
            <NoWalletAddressHeader>
                Please select a wallet above or import a wallet to get started.
            </NoWalletAddressHeader>
        );
    }

    return (
        <AddressHeader
            address={currentWallet.userId + ":" + currentWallet.publicKeyHex}
            balance=""
            onRefresh={() => refreshCurrentWallet()}
        />
    );
};

export { AddressHeader, StatefulAddressHeader };
