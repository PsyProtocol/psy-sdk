import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const TransactContainer = styled.div`
  padding: 16px 0;
`;

export const TransactTitle = styled.h2`
  color: ${config.theme.colors.text};
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 8px 0;
`;

export const TransactForm = styled.div`
  .mantine-TextInput-label,
  .mantine-NumberInput-label {
    color: ${config.theme.colors.text};
    font-weight: 500;
    margin-bottom: 4px;
  }
  
  .mantine-TextInput-input,
  .mantine-NumberInput-input {
    border-color: ${config.theme.colors.border};
    background-color: ${config.theme.colors.background};
    color: ${config.theme.colors.text};
    
    &:focus {
      border-color: ${config.theme.colors.primary};
    }
  }
  
  .mantine-Button-root {
    background-color: ${config.theme.colors.primary};
    border-color: ${config.theme.colors.primary};
    
    &:hover {
      background-color: ${config.theme.colors.accent};
      border-color: ${config.theme.colors.accent};
    }
    
    &:disabled {
      background-color: ${config.theme.colors.border};
      border-color: ${config.theme.colors.border};
      opacity: 0.6;
    }
  }
  
  .mantine-Button-outline {
    color: ${config.theme.colors.text};
    background-color: transparent;
    border-color: ${config.theme.colors.border};
    
    &:hover {
      background-color: rgba(115, 231, 255, 0.1);
    }
  }
`;