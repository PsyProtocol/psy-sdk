import React, { useState } from 'react';
import { Modal, TextInput, Button, Group, NumberInput, Text } from '@mantine/core';
import { useWalletConfig } from '../../config';
import { TransactContainer, TransactForm, TransactTitle } from './TransactModal.styles';

export type TransactType = 'transfer' | 'mint' | 'claim';

interface TransactModalProps {
  opened: boolean;
  onClose: () => void;
  type: TransactType;
  onConfirm?: (data: any) => void;
}

interface TransactConfig {
  title: string;
  description: string;
  fields: {
    recipient?: boolean;
    amount?: boolean;
    token?: boolean;
    memo?: boolean;
  };
  confirmText: string;
}

const transactConfigs: Record<TransactType, TransactConfig> = {
  transfer: {
    title: 'Transfer Tokens',
    description: 'Send tokens to another wallet address',
    fields: { recipient: true, amount: true, memo: true },
    confirmText: 'Send Transfer'
  },
  mint: {
    title: 'Mint Tokens',
    description: 'Create new tokens on the network',
    fields: { amount: true, token: true },
    confirmText: 'Mint Tokens'
  },
  claim: {
    title: 'Claim Rewards',
    description: 'Claim your available rewards',
    fields: { amount: true },
    confirmText: 'Claim Rewards'
  }
};

export const TransactModal: React.FC<TransactModalProps> = ({
  opened,
  onClose,
  type,
  onConfirm
}) => {
  const { config } = useWalletConfig();
  const [formData, setFormData] = useState({
    recipient: '',
    amount: 0,
    token: 'PSY',
    memo: ''
  });

  const transactConfig = transactConfigs[type];

  const handleSubmit = () => {
    if (onConfirm) {
      onConfirm(formData);
    }
    onClose();
    // Reset form
    setFormData({
      recipient: '',
      amount: 0,
      token: 'PSY',
      memo: ''
    });
  };

  const handleClose = () => {
    onClose();
    // Reset form
    setFormData({
      recipient: '',
      amount: 0,
      token: 'PSY',
      memo: ''
    });
  };

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title={transactConfig.title}
      size="md"
      centered
    >
      <TransactContainer>
        <TransactTitle>{transactConfig.title}</TransactTitle>
        <Text size="sm" c="dimmed" mb="lg">
          {transactConfig.description}
        </Text>

        <TransactForm>
          {transactConfig.fields.recipient && (
            <TextInput
              label="Recipient Address"
              placeholder="Enter wallet address"
              value={formData.recipient}
              onChange={(event) => setFormData({ ...formData, recipient: event.target.value })}
              mb="md"
              required
            />
          )}

          {transactConfig.fields.amount && (
            <NumberInput
              label="Amount"
              placeholder="0.00"
              value={formData.amount}
              onChange={(value) => setFormData({ ...formData, amount: value || 0 })}
              mb="md"
              min={0}
              step={0.01}
              precision={2}
              required
            />
          )}

          {transactConfig.fields.token && (
            <TextInput
              label="Token"
              value={formData.token}
              onChange={(event) => setFormData({ ...formData, token: event.target.value })}
              mb="md"
            />
          )}

          {transactConfig.fields.memo && (
            <TextInput
              label="Memo (Optional)"
              placeholder="Add a note"
              value={formData.memo}
              onChange={(event) => setFormData({ ...formData, memo: event.target.value })}
              mb="lg"
            />
          )}

          <Group justify="flex-end" gap="sm">
            <Button variant="outline" onClick={handleClose}>
              Cancel
            </Button>
            <Button 
              onClick={handleSubmit}
              disabled={
                (transactConfig.fields.recipient && !formData.recipient) ||
                (transactConfig.fields.amount && formData.amount <= 0)
              }
            >
              {transactConfig.confirmText}
            </Button>
          </Group>
        </TransactForm>
      </TransactContainer>
    </Modal>
  );
};

export default TransactModal;