import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const TransactContainer = styled.div`
  padding: 16px 0;
  position: relative;
  min-height: 200px;
  transition: all 0.3s ease;
`;

export const LoadingAnimation = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
  animation: fadeIn 0.4s ease-out;
  
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
`;

export const TransactionSteps = styled.div`
  margin: 20px 0;
`;

export const StepItem = styled.div<{ active?: boolean; completed?: boolean }>`
  display: flex;
  align-items: center;
  padding: 8px 0;
  color: ${props => {
    if (props.completed) return config.theme.colors.success || '#00C851';
    if (props.active) return config.theme.colors.primary;
    return config.theme.colors.textSecondary || '#666';
  }};
  
  .step-icon {
    margin-right: 12px;
    width: 20px;
    height: 20px;
    
    svg {
      transition: all 0.3s ease;
    }
  }
  
  .step-text {
    font-size: 14px;
    font-weight: ${props => props.active ? '600' : '400'};
  }
`;

export const SuccessAnimation = styled.div`
  text-align: center;
  padding: 40px 20px;
  animation: fadeIn 0.4s ease-out;
  
  .success-icon {
    color: ${config.theme.colors.success || '#00C851'};
    margin-bottom: 16px;
    
    svg {
      animation: successPulse 0.6s ease-out;
    }
  }
  
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  @keyframes successPulse {
    0% {
      transform: scale(0.8);
      opacity: 0;
    }
    50% {
      transform: scale(1.1);
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
`;

export const ErrorAnimation = styled.div`
  text-align: center;
  padding: 40px 20px;
  animation: fadeIn 0.4s ease-out;
  
  .error-icon {
    color: ${config.theme.colors.error || '#ff6b6b'};
    margin-bottom: 16px;
    
    svg {
      animation: errorShake 0.5s ease-out;
    }
  }
  
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  @keyframes errorShake {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-2px); }
    20%, 40%, 60%, 80% { transform: translateX(2px); }
  }
`;

export const TransactTitle = styled.h2`
  color: ${config.theme.colors.text};
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 8px 0;
`;

export const TransactForm = styled.div`
  animation: fadeIn 0.3s ease-out;
  
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(5px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  
  .mantine-TextInput-label,
  .mantine-NumberInput-label,
  .mantine-Select-label {
    color: ${config.theme.colors.text};
    font-weight: 500;
    margin-bottom: 4px;
  }
  
  .mantine-TextInput-input,
  .mantine-NumberInput-input,
  .mantine-Select-input {
    border-color: ${config.theme.colors.border};
    background-color: ${config.theme.colors.background};
    color: ${config.theme.colors.text};
    transition: all 0.2s ease;
    
    &:focus {
      border-color: ${config.theme.colors.primary};
      box-shadow: 0 0 0 2px rgba(115, 231, 255, 0.2);
    }
  }
  
  .mantine-Button-root {
    background-color: ${config.theme.colors.primary};
    border-color: ${config.theme.colors.primary};
    transition: all 0.2s ease;
    transform: translateY(0);
    
    &:hover:not(:disabled) {
      background-color: ${config.theme.colors.accent};
      border-color: ${config.theme.colors.accent};
      transform: translateY(-1px);
      box-shadow: 0 4px 8px rgba(115, 231, 255, 0.3);
    }
    
    &:active:not(:disabled) {
      transform: translateY(0);
    }
    
    &:disabled {
      background-color: ${config.theme.colors.border};
      border-color: ${config.theme.colors.border};
      opacity: 0.6;
      transform: translateY(0);
    }
  }
  
  .mantine-Button-outline {
    color: ${config.theme.colors.text};
    background-color: transparent;
    border-color: ${config.theme.colors.border};
    transition: all 0.2s ease;
    
    &:hover:not(:disabled) {
      background-color: rgba(115, 231, 255, 0.1);
      border-color: ${config.theme.colors.primary};
      transform: translateY(-1px);
    }
    
    &:active:not(:disabled) {
      transform: translateY(0);
    }
  }
`;