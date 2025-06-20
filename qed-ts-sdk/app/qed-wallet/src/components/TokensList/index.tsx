import React from 'react';
import { Group, Text, Avatar } from '@mantine/core';
import { useWalletConfig } from '../../config';
import { 
  TokensContainer, 
  TokenItem, 
  TokenInfo, 
  TokenName, 
  TokenBalance,
  TokenValue,
  EmptyState
} from './TokensList.styles';

interface Token {
  id: string;
  name: string;
  symbol: string;
  balance: string;
  value?: string;
  icon?: string;
}

interface TokensListProps {
  tokens?: Token[];
}

export const TokensList: React.FC<TokensListProps> = ({ tokens = [] }) => {
  const { config } = useWalletConfig();

  if (tokens.length === 0) {
    return (
      <TokensContainer>
        <EmptyState>
          <Text size="sm" c="dimmed">No tokens found</Text>
        </EmptyState>
      </TokensContainer>
    );
  }

  return (
    <TokensContainer>
      {tokens.map((token) => (
        <TokenItem key={token.id}>
          <Group gap="md">
            <Avatar
              src={token.icon}
              alt={token.symbol}
              radius="xl"
              size={40}
              color={config.theme.colors.primary}
            >
              {token.symbol.charAt(0)}
            </Avatar>
            
            <TokenInfo>
              <TokenName>{token.name}</TokenName>
              <Text size="xs" c="dimmed">{token.symbol}</Text>
            </TokenInfo>
            
            <div style={{ textAlign: 'right' }}>
              <TokenBalance>{token.balance}</TokenBalance>
              {token.value && (
                <TokenValue>≈ ${token.value}</TokenValue>
              )}
            </div>
          </Group>
        </TokenItem>
      ))}
    </TokensContainer>
  );
};

export default TokensList;