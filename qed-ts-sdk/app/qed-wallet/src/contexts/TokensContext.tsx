import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

export interface Token {
  id: string;
  name: string;
  symbol: string;
  contractId: string;
  balance: string;
  decimals: number;
}

interface TokensContextType {
  tokens: Token[];
  addToken: (token: Omit<Token, 'id' | 'balance'>) => Token;
  removeToken: (tokenId: string) => void;
  updateTokenBalance: (tokenId: string, balance: string) => void;
  getTokenByContractId: (contractId: string) => Token | undefined;
  refreshTokenBalances: () => Promise<void>;
}

const TOKENS_STORAGE_KEY = 'psy_wallet_tokens';

const defaultTokens: Token[] = [
  {
    id: 'psy',
    name: 'PSY',
    symbol: 'PSY',
    contractId: '0',
    balance: '0',
    decimals: 9,
  },
];

const TokensContext = createContext<TokensContextType | undefined>(undefined);

export const useTokens = () => {
  const context = useContext(TokensContext);
  if (!context) {
    throw new Error('useTokens must be used within a TokensProvider');
  }
  return context;
};

interface TokensProviderProps {
  children: ReactNode;
}

export const TokensProvider: React.FC<TokensProviderProps> = ({ children }) => {
  const [tokens, setTokens] = useState<Token[]>(() => {
    // Initialize state with tokens from localStorage
    try {
      const stored = localStorage.getItem(TOKENS_STORAGE_KEY);
      console.log('TokensProvider initializing, localStorage data:', stored);
      if (stored) {
        const parsedTokens = JSON.parse(stored);
        const customTokens = parsedTokens.filter((token: Token) => 
          !defaultTokens.some(defaultToken => defaultToken.id === token.id)
        );
        const allTokens = [...defaultTokens, ...customTokens];
        console.log('Loaded tokens from localStorage:', allTokens);
        return allTokens;
      }
    } catch (error) {
      console.warn('Failed to load tokens:', error);
    }
    console.log('No stored tokens, using default:', defaultTokens);
    return defaultTokens;
  });

  // Debug: Log tokens changes
  useEffect(() => {
    console.log('TokensProvider tokens state changed:', tokens);
  }, [tokens]);

  // Save tokens to localStorage whenever tokens change
  useEffect(() => {
    try {
      // Only save custom tokens (not default ones)
      const customTokens = tokens.filter(token => 
        !defaultTokens.some(defaultToken => defaultToken.id === token.id)
      );
      console.log('Saving tokens to localStorage:', customTokens);
      localStorage.setItem(TOKENS_STORAGE_KEY, JSON.stringify(customTokens));
    } catch (error) {
      console.warn('Failed to save tokens:', error);
    }
  }, [tokens]);

  const addToken = (token: Omit<Token, 'id' | 'balance'>) => {
    const newToken: Token = {
      ...token,
      id: `${token.contractId}_${token.symbol}`.toLowerCase(),
      balance: '0',
    };

    console.log('Adding new token:', newToken);
    console.log('Current tokens before add:', tokens);

    // Check if token already exists
    if (tokens.some(t => t.id === newToken.id)) {
      console.error('Token already exists with ID:', newToken.id);
      throw new Error('Token already exists');
    }

    setTokens(prev => {
      const newTokens = [...prev, newToken];
      console.log('New tokens after add:', newTokens);
      // Force re-render by creating new array reference
      return [...newTokens];
    });
    return newToken;
  };

  const removeToken = (tokenId: string) => {
    // Don't allow removing default tokens
    if (defaultTokens.some(token => token.id === tokenId)) {
      throw new Error('Cannot remove default token');
    }

    setTokens(prev => prev.filter(token => token.id !== tokenId));
  };

  const updateTokenBalance = (tokenId: string, balance: string) => {
    setTokens(prev => prev.map(token => 
      token.id === tokenId ? { ...token, balance } : token
    ));
  };

  const getTokenByContractId = (contractId: string) => {
    return tokens.find(token => token.contractId === contractId);
  };

  const refreshTokenBalances = async () => {
    // TODO: Implement actual balance fetching from contracts
    console.log('Refreshing token balances...');
    // This would typically call contract methods to get current balances
  };

  const value = {
    tokens,
    addToken,
    removeToken,
    updateTokenBalance,
    getTokenByContractId,
    refreshTokenBalances,
  };

  return (
    <TokensContext.Provider value={value}>
      {children}
    </TokensContext.Provider>
  );
};