import React, { useState } from 'react';
import { 
  Modal, 
  TextInput, 
  Button, 
  Group, 
  Text, 
  Alert, 
  NumberInput,
  Stack,
  Card,
  ActionIcon,
  Divider,
  Textarea,
  Select
} from '@mantine/core';
import { IconInfoCircle, IconPlus, IconTrash, IconDownload, IconUpload } from '@tabler/icons-react';
import { useWalletConfig, NetworkConfig, RealmConfig, CoordinatorConfig, defaultConfig } from '../../config';
import { useTokens } from '../../contexts/TokensContext';
import { PsyJSON } from '@psy/psy-sdk';

interface NetworkSettingsProps {
  opened: boolean;
  onClose: () => void;
}

export const NetworkSettings: React.FC<NetworkSettingsProps> = ({
  opened,
  onClose,
}) => {
  const { config, updateConfig } = useWalletConfig();
  const { tokens } = useTokens();
  const [localConfig, setLocalConfig] = useState<NetworkConfig>(config.network);
  const [error, setError] = useState<string | null>(null);
  const [importText, setImportText] = useState('');

  const handleSave = () => {
    try {
      setError(null);
      updateConfig({ network: localConfig });
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save configuration');
    }
  };

  const handleClose = () => {
    setLocalConfig(config.network); // Reset to current config
    setError(null);
    setImportText('');
    onClose();
  };

  const addRealmConfig = () => {
    const newRealm: RealmConfig = {
      id: Math.max(...localConfig.realm_configs.map(r => r.id), 0) + 1,
      rpc_url: ["http://127.0.0.1:8546"]
    };
    setLocalConfig({
      ...localConfig,
      realm_configs: [...localConfig.realm_configs, newRealm]
    });
  };

  const removeRealmConfig = (index: number) => {
    setLocalConfig({
      ...localConfig,
      realm_configs: localConfig.realm_configs.filter((_, i) => i !== index)
    });
  };

  const updateRealmConfig = (index: number, field: keyof RealmConfig, value: any) => {
    const updatedRealms = [...localConfig.realm_configs];
    if (field === 'rpc_url') {
      updatedRealms[index] = { ...updatedRealms[index], rpc_url: [value] };
    } else {
      updatedRealms[index] = { ...updatedRealms[index], [field]: value };
    }
    setLocalConfig({
      ...localConfig,
      realm_configs: updatedRealms
    });
  };

  const addCoordinatorConfig = () => {
    const newCoordinator: CoordinatorConfig = {
      id: Math.max(...localConfig.coordinator_configs.map(c => c.id), 0) + 1,
      rpc_url: ["http://127.0.0.1:8545"]
    };
    setLocalConfig({
      ...localConfig,
      coordinator_configs: [...localConfig.coordinator_configs, newCoordinator]
    });
  };

  const removeCoordinatorConfig = (index: number) => {
    setLocalConfig({
      ...localConfig,
      coordinator_configs: localConfig.coordinator_configs.filter((_, i) => i !== index)
    });
  };

  const updateCoordinatorConfig = (index: number, field: keyof CoordinatorConfig, value: any) => {
    const updatedCoordinators = [...localConfig.coordinator_configs];
    if (field === 'rpc_url') {
      updatedCoordinators[index] = { ...updatedCoordinators[index], rpc_url: [value] };
    } else {
      updatedCoordinators[index] = { ...updatedCoordinators[index], [field]: value };
    }
    setLocalConfig({
      ...localConfig,
      coordinator_configs: updatedCoordinators
    });
  };

  const exportConfig = () => {
    const configJson = PsyJSON.stringify(localConfig, null, 2);
    const blob = new Blob([configJson], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'network-config.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const importConfig = () => {
    try {
      const parsed = PsyJSON.parse(importText);
      // Validate the structure
      if (!parsed.users_per_realm || !parsed.realm_configs || !parsed.coordinator_configs) {
        throw new Error('Invalid configuration format');
      }
      setLocalConfig(parsed);
      setImportText('');
      setError(null);
    } catch (err) {
      setError('Invalid JSON configuration: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const resetToDefault = () => {
    console.log('Resetting network config to default');
    setLocalConfig(defaultConfig.network);
  };

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title="Network Settings"
      size="lg"
      centered
    >
      <Stack gap="md">
        {error && (
          <Alert variant="light" color="red" title="Configuration Error" icon={<IconInfoCircle />}>
            {error}
          </Alert>
        )}

        <NumberInput
          label="Users per Realm"
          labelProps={{ style: { fontSize: '14px', fontWeight: 500 } }}
          value={localConfig.users_per_realm}
          onChange={(value) => setLocalConfig({ ...localConfig, users_per_realm: value || 1048576 })}
          min={1}
          size="sm"
        />

        <Select
          label="Native Currency"
          labelProps={{ style: { fontSize: '14px', fontWeight: 500 } }}
          placeholder="Select native currency token"
          value={localConfig.nativeCurrency || '0'}
          onChange={(value) => setLocalConfig({ ...localConfig, nativeCurrency: value || '0' })}
          data={tokens.map(token => ({
            value: token.contractId,
            label: `${token.name} (${token.symbol}) - Contract ID: ${token.contractId}`,
          }))}
          size="sm"
          description="This token will be used as the default currency for gas fees and native operations"
        />

        <Divider label="Coordinator Configurations" labelPosition="center" labelProps={{ style: { fontSize: '14px' } }} />
        
        {localConfig.coordinator_configs.map((coordinator, index) => (
          <Card key={index} withBorder p="sm">
            <Group justify="space-between" mb="xs">
              <Text size="xs" fw={600}>Coordinator {index + 1}</Text>
              {localConfig.coordinator_configs.length > 1 && (
                <ActionIcon
                  variant="subtle"
                  color="red"
                  size="sm"
                  onClick={() => removeCoordinatorConfig(index)}
                >
                  <IconTrash size={14} />
                </ActionIcon>
              )}
            </Group>
            <Group grow>
              <NumberInput
                label="ID"
                labelProps={{ style: { fontSize: '13px', fontWeight: 500 } }}
                value={coordinator.id}
                onChange={(value) => updateCoordinatorConfig(index, 'id', value || 0)}
                min={0}
                size="xs"
              />
              <TextInput
                label="RPC URL"
                labelProps={{ style: { fontSize: '13px', fontWeight: 500 } }}
                value={coordinator.rpc_url[0] || ''}
                onChange={(e) => updateCoordinatorConfig(index, 'rpc_url', e.target.value)}
                placeholder="http://127.0.0.1:8545"
                size="xs"
              />
            </Group>
          </Card>
        ))}

        <Button
          leftSection={<IconPlus size={16} />}
          variant="outline"
          size="xs"
          onClick={addCoordinatorConfig}
        >
          Add Coordinator
        </Button>

        <Divider label="Realm Configurations" labelPosition="center" labelProps={{ style: { fontSize: '14px' } }} />
        
        {localConfig.realm_configs.map((realm, index) => (
          <Card key={index} withBorder p="sm">
            <Group justify="space-between" mb="xs">
              <Text size="xs" fw={600}>Realm {index + 1}</Text>
              {localConfig.realm_configs.length > 1 && (
                <ActionIcon
                  variant="subtle"
                  color="red"
                  size="sm"
                  onClick={() => removeRealmConfig(index)}
                >
                  <IconTrash size={14} />
                </ActionIcon>
              )}
            </Group>
            <Group grow>
              <NumberInput
                label="ID"
                labelProps={{ style: { fontSize: '13px', fontWeight: 500 } }}
                value={realm.id}
                onChange={(value) => updateRealmConfig(index, 'id', value || 0)}
                min={0}
                size="xs"
              />
              <TextInput
                label="RPC URL"
                labelProps={{ style: { fontSize: '13px', fontWeight: 500 } }}
                value={realm.rpc_url[0] || ''}
                onChange={(e) => updateRealmConfig(index, 'rpc_url', e.target.value)}
                placeholder="http://127.0.0.1:8546"
                size="xs"
              />
            </Group>
          </Card>
        ))}

        <Button
          leftSection={<IconPlus size={16} />}
          variant="outline"
          size="xs"
          onClick={addRealmConfig}
        >
          Add Realm
        </Button>

        <Divider label="Import/Export Configuration" labelPosition="center" labelProps={{ style: { fontSize: '14px' } }} />
        
        <Group grow>
          <Button
            leftSection={<IconDownload size={14} />}
            variant="outline"
            onClick={exportConfig}
            size="xs"
            styles={{ label: { fontSize: '12px' } }}
          >
            Export Config
          </Button>
          <Button
            leftSection={<IconUpload size={14} />}
            variant="outline"
            color="orange"
            onClick={resetToDefault}
            size="xs"
            styles={{ label: { fontSize: '12px' } }}
          >
            Reset to Default
          </Button>
        </Group>

        <Textarea
          label="Import Configuration (JSON)"
          labelProps={{ style: { fontSize: '14px', fontWeight: 500 } }}
          placeholder="Paste your network configuration JSON here..."
          value={importText}
          onChange={(e) => setImportText(e.target.value)}
          rows={3}
          size="sm"
        />

        <Button
          leftSection={<IconUpload size={14} />}
          onClick={importConfig}
          disabled={!importText.trim()}
          size="xs"
          styles={{ label: { fontSize: '12px' } }}
        >
          Import Configuration
        </Button>

        <Group justify="flex-end" gap="sm">
          <Button variant="outline" onClick={handleClose} size="xs" styles={{ label: { fontSize: '12px' } }}>
            Cancel
          </Button>
          <Button onClick={handleSave} size="xs" styles={{ label: { fontSize: '12px' } }}>
            Save Configuration
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
};

export default NetworkSettings;