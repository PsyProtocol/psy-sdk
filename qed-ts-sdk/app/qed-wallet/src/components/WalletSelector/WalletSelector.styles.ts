import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const WalletSelectorContainer = styled.div`
  .mantine-UnstyledButton-root {
    padding: 8px 12px;
    border-radius: 8px;
    transition: background-color 0.2s;
    cursor: pointer;
    
    &:hover {
      background-color: rgba(115, 231, 255, 0.1);
    }
  }
  
  .mantine-Menu-dropdown {
    background-color: ${config.theme.colors.background};
    border: 1px solid ${config.theme.colors.border};
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
  
  .mantine-Menu-label {
    color: ${config.theme.colors.text};
    font-weight: 600;
  }
  
  .mantine-Menu-item {
    color: ${config.theme.colors.text};
    
    &:hover {
      background-color: rgba(115, 231, 255, 0.1);
    }
  }
  
  .mantine-Menu-divider {
    border-color: ${config.theme.colors.border};
  }
`;

export const WalletInfo = styled.div`
  flex: 1;
  min-width: 0;
`;

export const WalletName = styled.span`
  color: ${config.theme.colors.text};
  font-weight: 500;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
`;

export const ChevronIcon = styled.div`
  color: ${config.theme.colors.text};
  display: flex;
  align-items: center;
`;