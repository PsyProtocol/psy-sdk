import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const ExtensionContainer = styled.div`
  background-color: ${config.theme.colors.background};
  width: ${config.extension.width}px;
  height: ${config.extension.height}px;
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  
  /* Very simple approach - just set the container properties */
  > div {
    height: 100%;
    width: 100%;
    padding: 0 !important;
    position: relative;
    background-color: ${config.theme.colors.background};
    overflow: hidden;
  }
`;

export const LoadingContainer = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  width: ${config.extension.width}px;
  height: ${config.extension.height}px;
  background-color: ${config.theme.colors.background};
  color: ${config.theme.colors.text};
  font-size: 16px;
`;

export const LoadingContent = styled.div`
  text-align: center;
`;

export const Logo = styled.img`
  width: 60px;
  margin-bottom: 20px;
`;

export const ErrorContainer = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  width: ${config.extension.width}px;
  height: ${config.extension.height}px;
  background-color: ${config.theme.colors.background};
  color: ${config.theme.colors.text};
  padding: 20px;
  text-align: center;
`;

export const Header = styled.div`
  padding: 16px 20px;
  border-bottom: 1px solid ${config.theme.colors.border};
  display: flex;
  align-items: center;
  justify-content: space-between;
  background-color: ${config.theme.colors.background};
  position: relative;
  z-index: 100;
`;

export const HeaderLeft = styled.div`
  display: flex;
  align-items: center;
  flex: 1;
`;

export const HeaderRight = styled.div`
  display: flex;
  align-items: center;
`;

export const SettingsButton = styled.button`
  background: none;
  border: none;
  color: ${config.theme.colors.text};
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.1);
    transform: scale(1.05);
  }
`;

export const MainContent = styled.div`
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  background-color: ${config.theme.colors.background};
  position: relative;
  margin-bottom: 60px; /* Account for bottom navigation */
`;

export const ErrorTitle = styled.h3`
  margin: 0 0 10px 0;
`;

export const ErrorMessage = styled.p`
  margin: 0 0 10px 0;
`;

export const ErrorHint = styled.p`
  font-size: 12px;
  margin-top: 10px;
  margin-bottom: 0;
`;