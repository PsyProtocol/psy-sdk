import React from 'react';
import { Group } from '@mantine/core';
import { IconSend, IconCoins, IconGift } from '@tabler/icons-react';
import { useWalletConfig } from '../../config';
import { 
  ActionButtonsContainer, 
  ActionButton, 
  ActionIcon, 
  ActionLabel 
} from './ActionButtons.styles';

interface ActionButtonsProps {
  onTransfer?: () => void;
  onMint?: () => void;
  onClaim?: () => void;
}

export const ActionButtons: React.FC<ActionButtonsProps> = ({
  onTransfer,
  onMint,
  onClaim,
}) => {
  return (
    <ActionButtonsContainer>
      <Group justify="space-around" gap={0}>
        <ActionButton onClick={onTransfer}>
          <ActionIcon>
            <IconSend size={24} />
          </ActionIcon>
          <ActionLabel>Transfer</ActionLabel>
        </ActionButton>
        
        <ActionButton onClick={onMint}>
          <ActionIcon>
            <IconCoins size={24} />
          </ActionIcon>
          <ActionLabel>Mint</ActionLabel>
        </ActionButton>
        
        <ActionButton onClick={onClaim}>
          <ActionIcon>
            <IconGift size={24} />
          </ActionIcon>
          <ActionLabel>Claim</ActionLabel>
        </ActionButton>
      </Group>
    </ActionButtonsContainer>
  );
};

export default ActionButtons;