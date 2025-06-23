import React, { useState } from 'react';
import { Modal, TextInput, Button, Group, Alert, NumberInput } from '@mantine/core';
import { IconInfoCircle } from '@tabler/icons-react';
import { useTokens } from '../../contexts/TokensContext';

interface TokenImportModalProps {
  opened: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export const TokenImportModal: React.FC<TokenImportModalProps> = ({
  opened,
  onClose,
  onSuccess,
}) => {
  const { addToken } = useTokens();
  
  const [formData, setFormData] = useState({
    contractId: '',
    name: '',
    symbol: '',
    decimals: 9,
  });

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    if (!formData.contractId || !formData.name || !formData.symbol) {
      setError('All fields are required');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      // Validate contract ID (should be a valid number)
      const contractIdNum = parseInt(formData.contractId);
      if (isNaN(contractIdNum) || contractIdNum < 0) {
        setError('Contract ID must be a valid positive number');
        return;
      }

      // Add the token
      addToken({
        contractId: formData.contractId,
        name: formData.name,
        symbol: formData.symbol.toUpperCase(),
        decimals: formData.decimals,
      });

      console.log('Token imported successfully:', formData);
      
      if (onSuccess) {
        onSuccess();
      }

      handleClose();
    } catch (err) {
      console.error('Failed to import token:', err);
      setError(err instanceof Error ? err.message : 'Failed to import token');
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    onClose();
    setError(null);
    setFormData({
      contractId: '',
      name: '',
      symbol: '',
      decimals: 9,
    });
  };

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title="Import Token"
      size="md"
      centered
    >
      <div style={{ padding: '0 4px' }}>
        {error && (
          <Alert variant="light" color="red" title="Import Error" icon={<IconInfoCircle />} mb="md">
            {error}
          </Alert>
        )}

        <TextInput
          label="Contract ID"
          placeholder="Enter contract ID (e.g., 123)"
          value={formData.contractId}
          onChange={(event) => {
            setFormData({ ...formData, contractId: event.target.value });
            if (error) setError(null);
          }}
          mb="md"
          required
        />

        <TextInput
          label="Token Name"
          placeholder="Enter token name (e.g., My Token)"
          value={formData.name}
          onChange={(event) => {
            setFormData({ ...formData, name: event.target.value });
            if (error) setError(null);
          }}
          mb="md"
          required
        />

        <TextInput
          label="Symbol"
          placeholder="Enter token symbol (e.g., MTK)"
          value={formData.symbol}
          onChange={(event) => {
            setFormData({ ...formData, symbol: event.target.value.toUpperCase() });
            if (error) setError(null);
          }}
          mb="md"
          required
        />

        <NumberInput
          label="Decimals"
          placeholder="Token decimals"
          value={formData.decimals}
          onChange={(value) => setFormData({ ...formData, decimals: value || 9 })}
          min={0}
          max={18}
          mb="lg"
        />

        <Group justify="flex-end" gap="sm">
          <Button variant="outline" onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button 
            onClick={handleSubmit}
            disabled={loading || !formData.contractId || !formData.name || !formData.symbol}
          >
            {loading ? 'Importing...' : 'Import Token'}
          </Button>
        </Group>
      </div>
    </Modal>
  );
};

export default TokenImportModal;