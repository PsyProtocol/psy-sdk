import React, { useEffect, useMemo, useState } from "react";
import styles from "./QedWalletWidget.module.scss";
import { useWalletState } from "../../hooks/useWalletState";
import { StatefulAddressSelector } from "../AddressSelector";
import { AddressModal } from "../AddressModal";
import { StatefulAddressHeader } from "../AddressHeader";
import { WalletActions } from "../WalletActions";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";

interface IQedWalletWidgetProps {
    className?: string;
    provider: QedUserWalletProvider;
    children?: React.ReactNode;
}
const QedWalletWidget: React.FC<IQedWalletWidgetProps> = ({ className, provider, children }) => {
    const [setWalletProvider] = useWalletState((state) => [state.setWalletProvider]);

    useEffect(() => {
        setWalletProvider(provider);
    }, [provider]);
    return (
        <div className={styles.walletWidget + (className ? " " + className : "")}>
            <div className={styles.walletWidgetHeader}>
                <StatefulAddressSelector />
            </div>
            <div className={styles.walletWidgetBody}>
                <StatefulAddressHeader />
                <WalletActions />
                {children}
            </div>
            <AddressModal />
        </div>
    );
};

export { QedWalletWidget };
