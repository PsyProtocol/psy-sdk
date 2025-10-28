import React, { createContext, useContext, ReactNode } from 'react';
import { ThemeProvider as StyledThemeProvider } from 'styled-components';
import { lightTheme, darkTheme, extensionTheme, WalletTheme } from './index';

interface ThemeContextType {
  theme: WalletTheme;
  themeMode: 'light' | 'dark' | 'extension';
  setThemeMode: (mode: 'light' | 'dark' | 'extension') => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export const useTheme = () => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

interface ThemeProviderProps {
  children: ReactNode;
  defaultTheme?: 'light' | 'dark' | 'extension';
}

export const WalletThemeProvider: React.FC<ThemeProviderProps> = ({ 
  children, 
  defaultTheme = 'dark' 
}) => {
  const [themeMode, setThemeMode] = React.useState<'light' | 'dark' | 'extension'>(defaultTheme);

  const theme = React.useMemo(() => {
    switch (themeMode) {
      case 'light':
        return lightTheme;
      case 'dark':
        return darkTheme;
      case 'extension':
        return extensionTheme;
      default:
        return darkTheme;
    }
  }, [themeMode]);

  const contextValue = React.useMemo(
    () => ({
      theme,
      themeMode,
      setThemeMode,
    }),
    [theme, themeMode]
  );

  return (
    <ThemeContext.Provider value={contextValue}>
      <StyledThemeProvider theme={theme}>
        {children}
      </StyledThemeProvider>
    </ThemeContext.Provider>
  );
};