import React, { useMemo, useCallback, useState, useEffect } from "react";
import { Combobox, Group, Input, InputBase, Text, useCombobox } from "@mantine/core";
import { BlokiesIcon } from "@qed/blokies-react";
import { getNetworkNameById } from "../../utils/network";
import { useWalletState } from "../../hooks/useWalletState";
import { TfiImport, TfiPlus, TfiReload } from "react-icons/tfi";
import { formatBalance } from "../../utils/balance";
import { AddressModalType, useAddressModal } from "../../hooks/useAddressModal";
import { sha256Buffer } from "@qed/utils";
import { NetworkId } from "@qed/qed-sdk/src/action";
import { useWalletConfig } from "../../config";
import { useBlockNumber, useUserBalance } from "../../utils/data";

interface IAddressSelectorBaseProps {
    className?: string;
    onWalletManagmentAction?: (action: WalletManagmentAction) => void;
}

interface IAddressSelectorItem {
    address: string;
    networkId: NetworkId;
    balanceString?: string;
    blockNumber?: number;
}
type WalletManagmentAction = "new-wallet" | "import-wallet" | "refresh-wallets";
interface IControlledAddressSelectorProps extends IAddressSelectorBaseProps {
    address: string;
    onChange: (address: string) => void;
    options: {
        address: string;
        networkId: NetworkId;
        balanceString?: string;
        blockNumber?: number;
    }[];
    showAddNew?: boolean;
    showImport?: boolean;
}
interface IStatefulAddressSelectorProps extends IAddressSelectorBaseProps { }

function SelectOption({ address, networkId, balanceString }: IAddressSelectorItem) {
    const { provider, currentWallet } = useWalletState((state) => ({
        provider: state.provider,
        currentWallet: state.currentWallet,
    }));

    const { getNativeCurrency } = useWalletConfig();
    const contractId = parseInt(getNativeCurrency(), 10);
    // refresh checkpoint every 10 seconds
    const currentBlockNumber = useBlockNumber(provider, 10000);
    const currentAddress = !currentWallet ? address : `${address}: ${currentWallet.publicKeyHex}`;
    const balance = useUserBalance(provider, currentBlockNumber, parseInt(address), contractId, 10000);
    console.log("userId:", parseInt(address));
    console.log("balance:", balance);
    sha256Buffer;
    return (
        <Group>
            <BlokiesIcon
                seed={sha256Buffer(new TextEncoder().encode("city-rollup:" + address), "hex")}
                size={8}
                scale={4}
            />
            <div>
                <Text fz="sm" fw={500}>
                    {currentAddress}
                </Text>
                <Text fz="xs" opacity={0.6}>
                    {getNetworkNameById(networkId)} {typeof balanceString === "string" ? " - " + balance.toString() : ""}
                    {currentBlockNumber !== null && ` - Checkpoint: ${currentBlockNumber}`}
                </Text>
            </div>
        </Group>
    );
}

const ControlledAddressSelector: React.FC<IControlledAddressSelectorProps> = ({
    address,
    onChange,
    options,
    showAddNew,
    showImport,
    onWalletManagmentAction,
}) => {
    const combobox = useCombobox({
        onDropdownClose: () => combobox.resetSelectedOption(),
    });

    const selectedOption = options.find((option) => option.address === address) || null;

    const comboOptions: React.JSX.Element[] = useMemo(() => {
        const addressOptions = options.map((item) => (
            <Combobox.Option value={item.address} key={item.address}>
                <SelectOption {...item} />
            </Combobox.Option>
        ));
        return addressOptions.concat([
            <Combobox.Group label="Wallet Management" key="wallet-management" style={{ color: 'black' }}>
                {showAddNew ? (
                    <Combobox.Option value="new-wallet">
                        <Group>
                            <TfiPlus size={20} />
                            <div>
                                <Text fz="sm" fw={500}>
                                    New Wallet...
                                </Text>
                                <Text fz="xs" opacity={0.6}>
                                    Create a new Qed wallet.
                                </Text>
                            </div>
                        </Group>
                    </Combobox.Option>
                ) : null}
                {showImport ? (
                    <Combobox.Option value="import-wallet" style={{ color: 'black' }}>
                        <Group>
                            <TfiImport size={20} />
                            <div>
                                <Text fz="sm" fw={500}>
                                    Import Wallet...
                                </Text>
                                <Text fz="xs" opacity={0.6}>
                                    Import an existing wallet
                                </Text>
                            </div>
                        </Group>
                    </Combobox.Option>
                ) : null}

                <Combobox.Option value="refresh-wallets" style={{ color: 'black' }}>
                    <Group>
                        <TfiReload size={20} />
                        <div>
                            <Text fz="sm" fw={500}>
                                Refresh Wallets
                            </Text>
                            <Text fz="xs" opacity={0.6}>
                                Refresh the list of wallets.
                            </Text>
                        </div>
                    </Group>
                </Combobox.Option>
            </Combobox.Group>,
        ]);
    }, [options, showAddNew, showImport]);

    return (
        <Combobox
            store={combobox}
            withinPortal={false}
            onOptionSubmit={(val) => {
                if (val === "new-wallet" || val === "import-wallet" || val === "refresh-wallets") {
                    if (typeof onWalletManagmentAction === "function") {
                        onWalletManagmentAction(val);
                        combobox.closeDropdown();
                    }
                } else {
                    onChange(val);
                    combobox.closeDropdown();
                }
            }}
        >
            <Combobox.Target>
                <InputBase
                    component="button"
                    type="button"
                    pointer
                    rightSection={<Combobox.Chevron />}
                    onClick={() => combobox.toggleDropdown()}
                    rightSectionPointerEvents="none"
                    multiline
                >
                    {selectedOption ? (
                        <SelectOption {...selectedOption} />
                    ) : (
                        <Input.Placeholder>Select an address...</Input.Placeholder>
                    )}
                </InputBase>
            </Combobox.Target>

            <Combobox.Dropdown>
                <Combobox.Options>{comboOptions}</Combobox.Options>
            </Combobox.Dropdown>
        </Combobox>
    );
};

const StatefulAddressSelector: React.FC<IStatefulAddressSelectorProps> = ({ className }) => {
    const [
        currency,
        currentWallet,
        wallets,
        walletAbilities,
        providerAbilities,
        setActiveWalletAsync,
        addRandomWallet,
        refreshAllWallets,
    ] = useWalletState((state) => [
        state.currency,
        state.currentWallet,
        state.wallets,
        state.walletAbilities,
        state.providerAbilities,
        state.setActiveWalletAsync,
        state.addRandomWallet,
        state.refreshAllWallets,
    ]);
    const [openModal] = useAddressModal((state) => [state.openModal]);
    return (
        <ControlledAddressSelector
            className={className}
            address={currentWallet?.userId.toString() || ""}
            onChange={(address) => {
                setActiveWalletAsync(parseFloat(address));
            }}
            options={wallets.map((wallet) => ({
                address: wallet.userId + "",
                networkId: wallet.networkId,
                balanceString: formatBalance(wallet.balance, currency),
                blockNumber: undefined
            }))}
            showAddNew={providerAbilities.includes("add-random-private-key")}
            showImport={providerAbilities.includes("import-private-key")}
            onWalletManagmentAction={(action) => {
                if (action === "new-wallet") {
                    addRandomWallet(true);
                } else if (action === "import-wallet") {
                    openModal(AddressModalType.Import, {});
                } else if (action === "refresh-wallets") {
                    refreshAllWallets().catch((err) => console.error("error refreshing wallets", err));
                }
            }}
        />
    );
};

export { StatefulAddressSelector, ControlledAddressSelector };
export type { WalletManagmentAction };
