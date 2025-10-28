import React, { useState } from 'react';
import { 
  Container, 
  Title, 
  TextInput, 
  Button, 
  ColorInput, 
  NumberInput,
  Switch,
  Tabs,
  Box,
  Group,
  Text,
  Divider
} from '@mantine/core';
import { IconSettings, IconPalette, IconNetwork, IconWallet, IconArrowLeft } from '@tabler/icons-react';
import { useWalletConfig, WalletConfig } from '../../config';
import { SettingsContainer, SettingsSection, SettingsTitle, BackButton } from './Settings.styles';
import { useNavigate } from 'react-router-dom';

export const Settings: React.FC = () => {
  const { config, updateConfig } = useWalletConfig();
  const [localConfig, setLocalConfig] = useState<WalletConfig>(config);
  const [hasChanges, setHasChanges] = useState(false);
  const navigate = useNavigate();

  const handleConfigChange = (section: keyof WalletConfig, field: string, value: any) => {
    const newConfig = {
      ...localConfig,
      [section]: {
        ...localConfig[section],
        [field]: value,
      },
    };
    setLocalConfig(newConfig);
    setHasChanges(true);
  };

  const handleColorChange = (field: string, value: string) => {
    const newConfig = {
      ...localConfig,
      theme: {
        ...localConfig.theme,
        colors: {
          ...localConfig.theme.colors,
          [field]: value,
        },
      },
    };
    setLocalConfig(newConfig);
    setHasChanges(true);
  };

  const handleSave = () => {
    updateConfig(localConfig);
    setHasChanges(false);
  };

  const handleReset = () => {
    setLocalConfig(config);
    setHasChanges(false);
  };

  return (
    <SettingsContainer>
      <Container size="md">
        <Group justify="space-between" align="center" mb="lg">
          <BackButton onClick={() => navigate('/')}>
            <IconArrowLeft size={20} />
          </BackButton>
          <SettingsTitle style={{ margin: 0 }}>Wallet Settings</SettingsTitle>
          <div style={{ width: 40 }} /> {/* Spacer for centering */}
        </Group>
        
        <Tabs defaultValue="theme" variant="outline">
          <Tabs.List>
            <Tabs.Tab value="theme" leftSection={<IconPalette size={16} />}>
              Theme
            </Tabs.Tab>
            <Tabs.Tab value="network" leftSection={<IconNetwork size={16} />}>
              Network
            </Tabs.Tab>
            <Tabs.Tab value="wallet" leftSection={<IconWallet size={16} />}>
              Wallet
            </Tabs.Tab>
            <Tabs.Tab value="extension" leftSection={<IconSettings size={16} />}>
              Extension
            </Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="theme" pt="lg">
            <SettingsSection>
              <Text size="lg" fw={600} mb="md">Color Theme</Text>
              
              <Group gap="md" mb="md">
                <ColorInput
                  label="Background Color"
                  value={localConfig.theme.colors.background}
                  onChange={(value) => handleColorChange('background', value)}
                  style={{ flex: 1 }}
                />
                <ColorInput
                  label="Text Color"
                  value={localConfig.theme.colors.text}
                  onChange={(value) => handleColorChange('text', value)}
                  style={{ flex: 1 }}
                />
              </Group>

              <Group gap="md" mb="md">
                <ColorInput
                  label="Primary Color"
                  value={localConfig.theme.colors.primary}
                  onChange={(value) => handleColorChange('primary', value)}
                  style={{ flex: 1 }}
                />
                <ColorInput
                  label="Border Color"
                  value={localConfig.theme.colors.border}
                  onChange={(value) => handleColorChange('border', value)}
                  style={{ flex: 1 }}
                />
              </Group>

              <Group gap="md">
                <ColorInput
                  label="Accent Color"
                  value={localConfig.theme.colors.accent}
                  onChange={(value) => handleColorChange('accent', value)}
                  style={{ flex: 1 }}
                />
                <ColorInput
                  label="Primary Text Color"
                  value={localConfig.theme.colors.primaryText}
                  onChange={(value) => handleColorChange('primaryText', value)}
                  style={{ flex: 1 }}
                />
              </Group>
            </SettingsSection>
          </Tabs.Panel>

          <Tabs.Panel value="network" pt="lg">
            <SettingsSection>
              <Text size="lg" fw={600} mb="md">Network Configuration</Text>
              
              <TextInput
                label="RPC URL"
                value={localConfig.network.rpcUrl}
                onChange={(event) => handleConfigChange('network', 'rpcUrl', event.currentTarget.value)}
                mb="md"
              />

              <Group gap="md" mb="md">
                <TextInput
                  label="Network ID"
                  value={localConfig.network.networkId}
                  onChange={(event) => handleConfigChange('network', 'networkId', event.currentTarget.value)}
                  style={{ flex: 1 }}
                />
                <NumberInput
                  label="Chain ID"
                  value={localConfig.network.chainId}
                  onChange={(value) => handleConfigChange('network', 'chainId', value)}
                  style={{ flex: 1 }}
                />
              </Group>

              <TextInput
                label="Network Name"
                value={localConfig.network.name}
                onChange={(event) => handleConfigChange('network', 'name', event.currentTarget.value)}
                mb="md"
              />
            </SettingsSection>
          </Tabs.Panel>

          <Tabs.Panel value="wallet" pt="lg">
            <SettingsSection>
              <Text size="lg" fw={600} mb="md">Wallet Configuration</Text>
              
              <TextInput
                label="Default Wallet Name"
                value={localConfig.wallet.defaultWalletName}
                onChange={(event) => handleConfigChange('wallet', 'defaultWalletName', event.currentTarget.value)}
                mb="md"
              />

              <Switch
                label="Enable Auto Refresh"
                checked={localConfig.wallet.enableAutoRefresh}
                onChange={(event) => handleConfigChange('wallet', 'enableAutoRefresh', event.currentTarget.checked)}
                mb="md"
              />

              <NumberInput
                label="Refresh Interval (milliseconds)"
                value={localConfig.wallet.refreshInterval}
                onChange={(value) => handleConfigChange('wallet', 'refreshInterval', value)}
                min={5000}
                max={300000}
                step={5000}
                disabled={!localConfig.wallet.enableAutoRefresh}
                mb="lg"
              />

              <Divider my="md" />
              
              <Text size="sm" c="dimmed" mb="sm">
                Danger Zone
              </Text>
              
              <Button
                color="red"
                variant="outline"
                onClick={() => {
                  if (confirm('Are you sure you want to clear all wallet data? This action cannot be undone.')) {
                    localStorage.removeItem('psy_wallet_data');
                    alert('Wallet data cleared. Please refresh the extension.');
                  }
                }}
              >
                Clear All Wallet Data
              </Button>
            </SettingsSection>
          </Tabs.Panel>

          <Tabs.Panel value="extension" pt="lg">
            <SettingsSection>
              <Text size="lg" fw={600} mb="md">Extension Settings</Text>
              
              <TextInput
                label="Extension Title"
                value={localConfig.extension.title}
                onChange={(event) => handleConfigChange('extension', 'title', event.currentTarget.value)}
                mb="md"
              />

              <Group gap="md">
                <NumberInput
                  label="Width (px)"
                  value={localConfig.extension.width}
                  onChange={(value) => handleConfigChange('extension', 'width', value)}
                  min={300}
                  max={800}
                  style={{ flex: 1 }}
                />
                <NumberInput
                  label="Height (px)"
                  value={localConfig.extension.height}
                  onChange={(value) => handleConfigChange('extension', 'height', value)}
                  min={400}
                  max={1000}
                  style={{ flex: 1 }}
                />
              </Group>
            </SettingsSection>
          </Tabs.Panel>
        </Tabs>

        <Divider my="xl" />

        <Group justify="flex-end">
          <Button 
            variant="outline" 
            onClick={handleReset}
            disabled={!hasChanges}
          >
            Reset
          </Button>
          <Button 
            onClick={handleSave}
            disabled={!hasChanges}
          >
            Save Settings
          </Button>
        </Group>
      </Container>
    </SettingsContainer>
  );
};

export default Settings;