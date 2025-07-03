import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const NavigationContainer = styled.div`
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background-color: ${config.theme.colors.background};
  border-top: 1px solid ${config.theme.colors.border};
  padding: 8px 0 16px 0;
  backdrop-filter: blur(10px);
  z-index: 1000;
`;

export const NavButton = styled.button<{ $active: boolean }>`
  background: none;
  border: none;
  cursor: pointer;
  padding: 8px 16px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  transition: all 0.2s;
  min-width: 60px;
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.1);
  }
`;

export const NavIcon = styled.div<{ $active: boolean }>`
  color: ${props => props.$active ? config.theme.colors.primary : config.theme.colors.text};
  opacity: ${props => props.$active ? 1 : 0.6};
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
`;

export const NavLabel = styled.span<{ $active: boolean }>`
  color: ${props => props.$active ? config.theme.colors.primary : config.theme.colors.text};
  opacity: ${props => props.$active ? 1 : 0.6};
  font-size: 10px;
  font-weight: ${props => props.$active ? 600 : 400};
  text-align: center;
  transition: all 0.2s;
`;