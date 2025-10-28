import React from 'react';
import { Group } from '@mantine/core';
import { IconHome, IconCoins } from '@tabler/icons-react';
import { useWalletConfig } from '../../config';
import { 
  NavigationContainer, 
  NavButton, 
  NavIcon, 
  NavLabel 
} from './BottomNavigation.styles';

interface BottomNavigationProps {
  activeTab: 'home' | 'tokens';
  onTabChange: (tab: 'home' | 'tokens') => void;
}

export const BottomNavigation: React.FC<BottomNavigationProps> = ({
  activeTab,
  onTabChange,
}) => {
  return (
    <NavigationContainer>
      <Group justify="space-around" gap={0}>
        <NavButton 
          $active={activeTab === 'home'} 
          onClick={() => onTabChange('home')}
        >
          <NavIcon $active={activeTab === 'home'}>
            <IconHome size={20} />
          </NavIcon>
          <NavLabel $active={activeTab === 'home'}>Home</NavLabel>
        </NavButton>
        
        <NavButton 
          $active={activeTab === 'tokens'} 
          onClick={() => onTabChange('tokens')}
        >
          <NavIcon $active={activeTab === 'tokens'}>
            <IconCoins size={20} />
          </NavIcon>
          <NavLabel $active={activeTab === 'tokens'}>Tokens</NavLabel>
        </NavButton>
      </Group>
    </NavigationContainer>
  );
};

export default BottomNavigation;