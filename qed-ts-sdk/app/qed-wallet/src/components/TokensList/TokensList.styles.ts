import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const TokensContainer = styled.div`
  padding: 16px;
  padding-bottom: 20px;
  height: 100%;
  overflow-y: auto;
`;

export const TokenItem = styled.div`
  padding: 12px 16px;
  border-radius: 12px;
  border: 1px solid ${config.theme.colors.border};
  margin-bottom: 8px;
  background-color: ${config.theme.colors.background};
  cursor: pointer;
  transition: all 0.2s;
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.05);
    border-color: ${config.theme.colors.primary};
  }
  
  &:last-child {
    margin-bottom: 0;
  }
`;

export const TokenInfo = styled.div`
  flex: 1;
  min-width: 0;
`;

export const TokenName = styled.div`
  color: ${config.theme.colors.text};
  font-weight: 500;
  font-size: 14px;
  margin: 0;
`;

export const TokenBalance = styled.div`
  color: ${config.theme.colors.text};
  font-weight: 600;
  font-size: 14px;
  margin: 0;
  text-align: right;
`;

export const TokenValue = styled.div`
  color: ${config.theme.colors.text};
  opacity: 0.6;
  font-size: 12px;
  margin: 0;
  text-align: right;
`;

export const EmptyState = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
`;