import React, { useState } from 'react';
import { Menu, UnstyledButton, Group, Avatar, Text, rem } from '@mantine/core';
import { IconChevronDown, IconPlus, IconUpload, IconRefresh } from '@tabler/icons-react';
import { BlokiesIcon } from '@qed/blokies-react';
import { sha256Buffer } from '@qed/utils';
import { useWalletConfig } from '../../config';
import { WalletSelectorContainer, WalletInfo, WalletName, ChevronIcon } from './WalletSelector.styles';

interface WalletSelectorProps {
  currentWallet?: {
    name: string;
    address: string;
    avatar?: string;
  };
  onNewWallet?: () => void;
  onImportWallet?: () => void;
  onRefreshWallets?: () => void;
}

export const WalletSelector: React.FC<WalletSelectorProps> = ({
  currentWallet,
  onNewWallet,
  onImportWallet,
  onRefreshWallets,
}) => {
  const { config } = useWalletConfig();
  const [opened, setOpened] = useState(false);

  const displayWallet = currentWallet || {
    name: config.wallet.defaultWalletName,
    address: 'No wallet',
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