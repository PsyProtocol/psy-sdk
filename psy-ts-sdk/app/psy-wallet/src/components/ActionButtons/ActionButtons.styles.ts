import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const ActionButtonsContainer = styled.div`
  padding: 20px 16px;
`;

export const ActionButton = styled.button`
  background: none;
  border: none;
  cursor: pointer;
  padding: 16px 12px;
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  transition: all 0.2s;
  min-width: 80px;
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.1);
    transform: translateY(-2px);
  }
  
  &:active {
    transform: translateY(0);
  }
`;

export const ActionIcon = styled.div`
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: linear-gradient(135deg, ${config.theme.colors.primary}, ${config.theme.colors.accent});
  display: flex;
  align-items: center;
  justify-content: center;
  color: ${config.theme.colors.primaryText};
  box-shadow: 0 4px 12px rgba(115, 231, 255, 0.3);
  transition: all 0.2s;
  
  ${ActionButton}:hover & {
    box-shadow: 0 6px 16px rgba(115, 231, 255, 0.4);
    transform: scale(1.05);
  }
`;

export const ActionLabel = styled.span`
  color: ${config.theme.colors.text};
  font-size: 12px;
  font-weight: 500;
  text-align: center;
`;