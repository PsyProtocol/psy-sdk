import type { DogeNetworkId, TWalletAbility } from "doge-sdk";
import React, { useMemo, useState } from "react";
import {
  Combobox,
  Group,
  Input,
  InputBase,
  Text,
  useCombobox,
} from "@mantine/core";
import { BlokiesIcon } from "@qstudio/blokies-react";
import { getNetworkNameById } from "../../utils/network";
import { useWalletState } from "../../hooks/useWalletState";
import { TfiImport, TfiPlus } from "react-icons/tfi";
import { formatBalance } from "../../utils/balance";
import { AddressModalType, useAddressModal } from "../../hooks/useAddressModal";

interface IAddressSelectorBaseProps {
  className?: string;
  onWalletManagmentAction?: (action: WalletManagmentAction) => void;
}

interface IAddressSelectorItem {
  address: string;
  networkId: DogeNetworkId;
  balanceString?: string;
}
type WalletManagmentAction = "new-wallet" | "import-wallet";
interface IControlledAddressSelectorProps extends IAddressSelectorBaseProps {
  address: string;
  onChange: (address: string) => void;
  options: {
    address: string;
    networkId: DogeNetworkId;
    balanceString?: string;
  }[];
  showAddNew?: boolean;
  showImport?: boolean;
}
interface IStatefulAddressSelectorProps extends IAddressSelectorBaseProps {}

function SelectOption({
  address,
  networkId,
  balanceString,
}: IAddressSelectorItem) {
  return (
    <Group>
      <BlokiesIcon seed={address} size={8} scale={4} />
      <div>
        <Text fz="sm" fw={500}>
          {address}
        </Text>
        <Text fz="xs" opacity={0.6}>
          {getNetworkNameById(networkId)}{" "}
          {typeof balanceString === "string" ? " - " + balanceString : ""}
        </Text>
      </div>
    </Group>
  );
}

const ControlledAddressSelector: React.FC<IControlledAddressSelectorProps> = ({
  className,
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

  const selectedOption =
    options.find((option) => option.address === address) || null;

  const comboOptions: React.JSX.Element[] = useMemo(() => {
    const addressOptions = options.map((item) => (
      <Combobox.Option value={item.address} key={item.address}>
        <SelectOption {...item} />
      </Combobox.Option>
    ));
    if (showAddNew || showImport) {
      return addressOptions.concat([
        <Combobox.Group label="Wallet Managment" key="wallet-management">
          {showAddNew ? (
            <Combobox.Option value="new-wallet">
              <Group>
                <TfiPlus size={20} />
                <div>
                  <Text fz="sm" fw={500}>
                    New Wallet...
                  </Text>
                  <Text fz="xs" opacity={0.6}>
                    Create a new Dogecoin wallet.
                  </Text>
                </div>
              </Group>
            </Combobox.Option>
          ) : null}
          {showImport ? (
            <Combobox.Option value="import-wallet">
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
        </Combobox.Group>,
      ]);
    } else {
      return addressOptions;
    }
  }, [options, showAddNew, showImport]);

  return (
    <Combobox
      store={combobox}
      withinPortal={false}
      onOptionSubmit={(val) => {
        if(val==="new-wallet" || val==="import-wallet"){
          if(typeof onWalletManagmentAction === "function"){
            onWalletManagmentAction(val);
            combobox.closeDropdown();
          }
        }else{

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

const StatefulAddressSelector: React.FC<IStatefulAddressSelectorProps> = ({
  className,
}) => {
  const [currency, currentWallet, wallets, abilities, setActiveWalletAsync, addRandomWallet] =
    useWalletState((state) => [
      state.currency,
      state.currentWallet,
      state.wallets,
      state.abilities,
      state.setActiveWalletAsync,
      state.addRandomWallet,
    ]);
  const [openModal]= useAddressModal((state)=>[state.openModal])
  return (
    <ControlledAddressSelector
      className={className}
      address={currentWallet?.address || ""}
      onChange={(address) => {
        setActiveWalletAsync(address);
      }}
      options={wallets.map((wallet) => ({
        address: wallet.address,
        networkId: wallet.networkId,
        balanceString: formatBalance(wallet.balance, currency),
      }))}
      showAddNew={abilities.includes("add-wallet-random")}
      showImport={
        abilities.includes("add-wallet-bip178") ||
        abilities.includes("add-wallet-bip39") ||
        abilities.includes("add-wallet-bip44")
      }
      onWalletManagmentAction={(action) => {
        if(action === "new-wallet"){
          addRandomWallet(true);
        }else if(action === "import-wallet"){
          openModal(AddressModalType.Import, {});
        }
      }}
    />
  );
};

export { StatefulAddressSelector, ControlledAddressSelector };
export type {
  WalletManagmentAction,
}