export interface WalletTheme {
  colors: {
    background: string;
    text: string;
    primary: string;
    primaryText: string;
    secondary: string;
    secondaryText: string;
    border: string;
    error: string;
    success: string;
    warning: string;
    info: string;
    // Extension specific colors
    extensionBg?: string;
    extensionText?: string;
  };
  fonts: {
    sans: string;
    mono: string;
  };
  spacing: {
    xs: string;
    sm: string;
    md: string;
    lg: string;
    xl: string;
  };
  borderRadius: {
    sm: string;
    md: string;
    lg: string;
  };
  shadows: {
    sm: string;
    md: string;
    lg: string;
  };
}

const baseTheme = {
  fonts: {
    sans: "'Reddit Sans', -apple-system, ui-sans-serif, system-ui, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'",
    mono: "'Reddit Mono', 'Fira Code', 'JetBrains Mono', 'SF Mono', 'Roboto Mono', Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace"
  },
  spacing: {
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px'
  },
  borderRadius: {
    sm: '4px',
    md: '8px',
    lg: '12px'
  },
  shadows: {
    sm: '0 1px 2px rgba(0, 0, 0, 0.1)',
    md: '0 4px 6px rgba(0, 0, 0, 0.1)',
    lg: '0 10px 15px rgba(0, 0, 0, 0.1)'
  }
};

export const lightTheme: WalletTheme = {
  ...baseTheme,
  colors: {
    background: '#f5f5f5',
    text: '#222222',
    primary: '#000000',
    primaryText: '#ffffff',
    secondary: '#666666',
    secondaryText: '#ffffff',
    border: '#e0e0e0',
    error: '#dc3545',
    success: '#28a745',
    warning: '#ffc107',
    info: '#17a2b8'
  }
};

export const darkTheme: WalletTheme = {
  ...baseTheme,
  colors: {
    background: '#2a2a2a',
    text: '#ffffff',
    primary: '#21517d',
    primaryText: '#ffffff',
    secondary: '#4a4a4a',
    secondaryText: '#ffffff',
    border: '#3a3a3a',
    error: '#dc3545',
    success: '#28a745',
    warning: '#ffc107',
    info: '#17a2b8'
  }
};

// Extension specific theme
export const extensionTheme: WalletTheme = {
  ...baseTheme,
  colors: {
    background: '#ffffff',
    text: '#73e7ff',
    primary: '#73e7ff',
    primaryText: '#ffffff',
    secondary: '#73e7ff',
    secondaryText: '#ffffff',
    border: '#73e7ff',
    error: '#ff6b6b',
    success: '#51cf66',
    warning: '#ffd43b',
    info: '#339af0',
    extensionBg: '#ffffff',
    extensionText: '#73e7ff'
  }
};