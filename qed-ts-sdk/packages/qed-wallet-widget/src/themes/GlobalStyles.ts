import { createGlobalStyle } from 'styled-components';
import { WalletTheme } from './index';

export const GlobalStyles = createGlobalStyle<{ theme: WalletTheme }>`
  * {
    box-sizing: border-box;
  }

  body {
    margin: 0;
    padding: 0;
    font-family: ${({ theme }) => theme.fonts.sans};
    background-color: ${({ theme }) => theme.colors.background};
    color: ${({ theme }) => theme.colors.text};
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  code, pre {
    font-family: ${({ theme }) => theme.fonts.mono};
  }

  /* Scrollbar styles */
  ::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  ::-webkit-scrollbar-track {
    background: ${({ theme }) => theme.colors.background};
  }

  ::-webkit-scrollbar-thumb {
    background: ${({ theme }) => theme.colors.border};
    border-radius: ${({ theme }) => theme.borderRadius.sm};
  }

  ::-webkit-scrollbar-thumb:hover {
    background: ${({ theme }) => theme.colors.secondary};
  }

  /* Override Mantine styles when used with styled-components */
  .mantine-Button-root {
    background-color: ${({ theme }) => theme.colors.primary} !important;
    color: ${({ theme }) => theme.colors.primaryText} !important;
    border: 1px solid ${({ theme }) => theme.colors.primary} !important;
    
    &:hover {
      background-color: ${({ theme }) => theme.colors.secondary} !important;
      border-color: ${({ theme }) => theme.colors.secondary} !important;
    }
  }

  .mantine-TextInput-input,
  .mantine-Textarea-input,
  .mantine-Select-input {
    background-color: ${({ theme }) => theme.colors.background} !important;
    color: ${({ theme }) => theme.colors.text} !important;
    border-color: ${({ theme }) => theme.colors.border} !important;
  }

  .mantine-Modal-content {
    background-color: ${({ theme }) => theme.colors.background} !important;
    color: ${({ theme }) => theme.colors.text} !important;
  }

  .mantine-Paper-root {
    background-color: ${({ theme }) => theme.colors.background} !important;
    border: 1px solid ${({ theme }) => theme.colors.border} !important;
  }
`;