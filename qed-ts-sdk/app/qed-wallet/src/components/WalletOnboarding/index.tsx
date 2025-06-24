import React from 'react';
import { Group, Button, Text } from '@mantine/core';
import { IconPlus, IconUpload } from '@tabler/icons-react';
import { useWalletConfig } from '../../config';
import logoImage from '../../assets/psy.png';
import {
  OnboardingContainer,
  OnboardingContent,
  OnboardingLogo,
  OnboardingTitle,
  OnboardingSubtitle,
  ActionCard,
  ActionIcon,
  ActionTitle,
  ActionDescription
} from './WalletOnboarding.styles';

interface WalletOnboardingProps {
  onCreateWallet: () => void;
  onImportWallet: () => void;
}

export const WalletOnboarding: React.FC<WalletOnboardingProps> = ({
  onCreateWallet,
  onImportWallet
}) => {
  const { config } = useWalletConfig();

  return (
    <OnboardingContainer>
      <OnboardingContent>
        <OnboardingLogo src={logoImage} alt="Psy Wallet" />
        <OnboardingTitle>{config.extension.title}</OnboardingTitle>
        <OnboardingSubtitle>
          Welcome to Psy Wallet! Get started by creating a new wallet or importing an existing one.
        </OnboardingSubtitle>

        <Group gap="lg" mt="xl" style={{ width: '100%' }}>
          <ActionCard onClick={onCreateWallet}>
            <ActionIcon>
              <IconPlus size={24} />
            </ActionIcon>
            <ActionTitle>Create New Wallet</ActionTitle>
            <ActionDescription>
              Generate a new wallet with a random private key
            </ActionDescription>
          </ActionCard>

          <ActionCard onClick={onImportWallet}>
            <ActionIcon>
              <IconUpload size={24} />
            </ActionIcon>
            <ActionTitle>Import Wallet</ActionTitle>
            <ActionDescription>
              Import an existing wallet using your private key
            </ActionDescription>
          </ActionCard>
        </Group>

        <Text size="xs" c="dimmed" mt="xl" ta="center">
          Your wallet data will be securely stored locally on your device
        </Text>
      </OnboardingContent>
    </OnboardingContainer>
  );
};

export default WalletOnboarding;