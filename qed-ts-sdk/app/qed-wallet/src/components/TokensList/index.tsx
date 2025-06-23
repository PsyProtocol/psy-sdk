import React, { useState, useEffect } from 'react';
import { Group, Text, Avatar, Button, ActionIcon } from '@mantine/core';
import { IconPlus, IconTrash } from '@tabler/icons-react';
import { useWalletConfig } from '../../config';
import { useTokens } from '../../contexts/TokensContext';
import TokenImportModal from '../TokenImportModal';
import { 
  TokensContainer, 
  TokenItem, 
  TokenInfo, 
  TokenName, 
  TokenBalance,
  TokenValue,
  EmptyState
} from './TokensList.styles';

interface TokensListProps {
  // tokens prop is optional now since we get them from the hook
  tokens?: never;
}

export const TokensList: React.FC<TokensListProps> = () => {
  const { config } = useWalletConfig();
  const { tokens, removeToken, refreshTokenBalances } = useTokens();
  const [importModalOpen, setImportModalOpen] = useState(false);

  // Debug: Log tokens in TokensList
  useEffect(() => {
    console.log('TokensList tokens updated:', tokens);
  }, [tokens]);

  const handleRemoveToken = async (tokenId: string) => {
    try {
      await removeToken(tokenId);
    } catch (error) {
      console.error('Failed to remove token:', error);
    }
  };

  return (
    <TokensContainer>
      <Group justify="space-between" mb="md">
        <Text size="lg" fw={600}>Tokens</Text>
        <Button
          leftSection={<IconPlus size={16} />}
          variant="outline"
          size="xs"
          onClick={() => setImportModalOpen(true)}
        >
          Import Token
        </Button>
      </Group>

      {tokens.length === 0 ? (
        <EmptyState>
          <Text size="sm" c="dimmed">No tokens found</Text>
          <Text size="xs" c="dimmed" mt="xs">Import a token to get started</Text>
        </EmptyState>
      ) : (
        tokens.map((token) => (
          <TokenItem key={token.id}>
            <Group gap="md" justify="space-between">
              <Group gap="md">
                <Avatar
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
                  <Text size="xs" c="dimmed">Contract: {token.contractId}</Text>
                </TokenInfo>
              </Group>
              
              <Group gap="xs">
                <div style={{ textAlign: 'right' }}>
                  <TokenBalance>{token.balance}</TokenBalance>
                  <Text size="xs" c="dimmed">{token.symbol}</Text>
                </div>
                
                {!['psy'].includes(token.id) && (
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    size="sm"
                    onClick={() => handleRemoveToken(token.id)}
                  >
                    <IconTrash size={14} />
                  </ActionIcon>
                )}
              </Group>
            </Group>
          </TokenItem>
        ))
      )}

      <TokenImportModal
        opened={importModalOpen}
        onClose={() => setImportModalOpen(false)}
        onSuccess={() => {
          console.log('Token import successful, closing modal');
          setImportModalOpen(false);
        }}
      />
    </TokensContainer>
  );
};

export default TokensList;