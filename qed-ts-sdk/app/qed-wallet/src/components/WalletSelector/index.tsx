import React, { useState } from 'react';
import { Menu, UnstyledButton, Group, Avatar, Text, rem } from '@mantine/core';
import { IconChevronDown, IconPlus, IconUpload, IconRefresh } from '@tabler/icons-react';
import { BlokiesIcon } from '@qed/blokies-react';
import { sha256Buffer } from '@qed/utils';
import { useWalletConfig } from '../../config';
import { WalletSelectorContainer, WalletInfo, WalletName, ChevronIcon } from './WalletSelector.styles';
import { IQedWidgetWallet } from '@qed/qed-wallet-widget';

interface WalletSelectorProps {
  wallets: IQedWidgetWallet[];
  currentWallet?: {
    name: string;
    address: string;
    avatar?: string;
  };
  onNewWallet?: () => void;
  onImportWallet?: () => void;
  onRefreshWallets?: () => void;
  onSelectWallet?: (userId: number) => void;
}

export const WalletSelector: React.FC<WalletSelectorProps> = ({
  wallets,
  currentWallet,
  onNewWallet,
  onImportWallet,
  onRefreshWallets,
  onSelectWallet
}) => {
  const { config } = useWalletConfig();
  const [opened, setOpened] = useState(false);

  const displayWallet = currentWallet || {
    name: config.wallet.defaultWalletName,
    address: '******',
    avatar: '',
  };

  // Generate Blokies seed for avatar
  const avatarSeed = displayWallet.address !== 'No wallet'
    ? sha256Buffer(new TextEncoder().encode("psy-wallet:" + displayWallet.address), "hex")
    : sha256Buffer(new TextEncoder().encode("psy-wallet:default"), "hex");

  return (
    <WalletSelectorContainer>
      <Menu opened={opened} onChange={setOpened} width={200} position="bottom-start">
        <Menu.Target>
          <UnstyledButton>
            <Group gap={8}>
              <BlokiesIcon
                seed={avatarSeed}
                size={6}
                scale={4}
                style={{ borderRadius: '50%' }}
              />
              <WalletInfo>
                <WalletName>{displayWallet.name}</WalletName>
                <WalletName>0x{displayWallet.address?.substring(0, 6)}</WalletName>
              </WalletInfo>
              <ChevronIcon
                style={{
                  transform: opened ? 'rotate(180deg)' : 'none',
                  transition: 'transform 0.2s'
                }}
              >
                <IconChevronDown size={16} />
              </ChevronIcon>
            </Group>
          </UnstyledButton>
        </Menu.Target>

        <Menu.Dropdown>
          {wallets.length > 0 && (
            <>
              <Menu.Label>All Wallets</Menu.Label>
              {wallets.map((wallet) => (
                <Menu.Item
                  key={wallet.address}
                  leftSection={
                    <BlokiesIcon
                      seed={sha256Buffer(new TextEncoder().encode("psy-wallet:" + wallet.address), "hex")}
                      size={4}
                      scale={3}
                      style={{ borderRadius: '50%' }}
                    />
                  }
                  onClick={() => {
                    onSelectWallet?.(Number(wallet.userId));
                  }}
                >
                  <Text size="sm">{wallet.name}</Text>
                  <Text size="sm">0x{wallet.address?.substring(0, 6)}</Text>
                </Menu.Item>
              ))}
              <Menu.Divider />
            </>
          )}

          <Menu.Label>Wallet Management</Menu.Label>
          <Menu.Item
            leftSection={<IconPlus style={{ width: rem(14), height: rem(14) }} />}
            onClick={onNewWallet}
          >
            Create New Wallet
          </Menu.Item>
          <Menu.Item
            leftSection={<IconUpload style={{ width: rem(14), height: rem(14) }} />}
            onClick={onImportWallet}
          >
            Import Wallet
          </Menu.Item>
          <Menu.Divider />
          <Menu.Item
            leftSection={<IconRefresh style={{ width: rem(14), height: rem(14) }} />}
            onClick={onRefreshWallets}
          >
            Refresh Wallets
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    </WalletSelectorContainer>
  );
};

export default WalletSelector;