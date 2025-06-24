import React, { useState, useEffect } from 'react';
import { Modal, TextInput, Button, Group, NumberInput, Text, Alert, LoadingOverlay, Select, Loader } from '@mantine/core';
import { IconInfoCircle, IconCheck, IconX, IconSend, IconCoins, IconCreditCard } from '@tabler/icons-react';
import { useWalletState } from '@qed/qed-wallet-widget';
import { useWalletConfig } from '../../config';
import { useTokens } from '../../contexts/TokensContext';
import { 
  TransactContainer, 
  TransactForm, 
  TransactTitle, 
  LoadingAnimation,
  TransactionSteps,
  StepItem,
  SuccessAnimation,
  ErrorAnimation
} from './TransactModal.styles';

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
  methodName: string;
  fields: {
    recipient?: boolean;
    sender?: boolean;
    amount?: boolean;
    token?: boolean;
  };
  confirmText: string;
  getInputs: (data: any, selectedToken?: any) => { contractId: bigint; inputs: bigint[] };
}

const transactConfigs: Record<TransactType, TransactConfig> = {
  transfer: {
    title: 'Transfer Tokens',
    description: 'Send tokens to another wallet address',
    methodName: 'simple_transfer',
    fields: { recipient: true, amount: true, token: true },
    confirmText: 'Send Transfer',
    getInputs: (data, selectedToken) => ({
      contractId: BigInt(selectedToken?.contractId || '0'),
      inputs: [BigInt(Math.floor(data.amount * Math.pow(10, selectedToken?.decimals || 9)))],
    }),
  },
  mint: {
    title: 'Mint Tokens',
    description: 'Create new tokens on the network',
    methodName: 'simple_mint',
    fields: { amount: true, token: true },
    confirmText: 'Mint Tokens',
    getInputs: (data, selectedToken) => ({
      contractId: BigInt(selectedToken?.contractId || '0'),
      inputs: [BigInt(Math.floor(data.amount * Math.pow(10, selectedToken?.decimals || 9)))],
    }),
  },
  claim: {
    title: 'Claim',
    description: 'Claim tokens from sender',
    methodName: 'simple_claim',
    fields: { sender: true, token: true },
    confirmText: 'Claim',
    getInputs: (data, selectedToken) => ({
      contractId: BigInt(selectedToken?.contractId || '0'),
      inputs: [], // Claim typically doesn't need amount input, just sender info
    }),
  }
};

export const TransactModal: React.FC<TransactModalProps> = ({
  opened,
  onClose,
  type,
  onConfirm
}) => {
  const { config } = useWalletConfig();
  const { tokens } = useTokens();
  const [currentWallet, refreshAllWallets] = useWalletState((state) => [
    state.currentWallet,
    state.refreshAllWallets,
  ]);

  const [formData, setFormData] = useState({
    recipient: '',
    sender: '',
    amount: 0,
    token: 'PSY'
  });

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [transactionStep, setTransactionStep] = useState<'idle' | 'preparing' | 'executing' | 'success' | 'error'>('idle');
  const [showSuccess, setShowSuccess] = useState(false);

  const transactConfig = transactConfigs[type];
  const selectedToken = tokens.find(token => token.symbol === formData.token);

  // Debug: Log tokens in modal
  useEffect(() => {
    console.log('TransactModal tokens:', tokens);
  }, [tokens]);

  const handleSubmit = async () => {
    if (!currentWallet) {
      setError('No wallet selected');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      setTransactionStep('preparing');

      // Step 1: Preparing transaction
      await new Promise(resolve => setTimeout(resolve, 800));
      
      const contractCall = transactConfig.getInputs(formData, selectedToken);

      console.log('Executing contract call:', {
        type,
        contractId: contractCall.contractId,
        methodName: transactConfig.methodName,
        formData,
        selectedToken,
        rawAmount: formData.amount,
        scaledAmount: contractCall.inputs[0]
      });

      // Step 2: Executing transaction
      setTransactionStep('executing');
      await new Promise(resolve => setTimeout(resolve, 500));

      // Execute the actual contract call
      await currentWallet.wallet.execContractCall(currentWallet.publicKeyHex, {
        contract_id: contractCall.contractId,
        method_name: transactConfig.methodName,
        inputs: contractCall.inputs,
      });

      // Step 3: Success
      setTransactionStep('success');
      setShowSuccess(true);

      // Refresh wallet state to update balance
      await refreshAllWallets();

      console.log(`${type} transaction completed successfully`);
      
      // Call the onConfirm callback with transaction data
      if (onConfirm) {
        onConfirm({
          type,
          ...formData,
          contractId: contractCall.contractId.toString(),
          methodName: transactConfig.methodName,
          selectedToken,
          scaledAmount: contractCall.inputs[0].toString(),
        });
      }

      // Show success for 2 seconds then close
      setTimeout(() => {
        handleClose();
      }, 2000);
    } catch (err) {
      console.error(`${type} transaction failed:`, err);
      setError(err instanceof Error ? err.message : 'Transaction failed');
      setTransactionStep('error');
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    onClose();
    setError(null);
    setTransactionStep('idle');
    setShowSuccess(false);
    // Reset form
    setFormData({
      recipient: '',
      sender: '',
      amount: 0,
      token: 'PSY'
    });
  };

  const getStepIcon = (step: string, currentStep: string) => {
    const isActive = currentStep === step;
    const isCompleted = ['preparing', 'executing', 'success'].indexOf(currentStep) > ['preparing', 'executing', 'success'].indexOf(step);
    
    if (isCompleted || (isActive && currentStep === 'success')) {
      return <IconCheck size={16} />;
    }
    
    if (isActive) {
      return <Loader size={16} />;
    }
    
    switch (step) {
      case 'preparing':
        return <IconCreditCard size={16} />;
      case 'executing':
        return <IconSend size={16} />;
      case 'success':
        return <IconCheck size={16} />;
      default:
        return <IconCoins size={16} />;
    }
  };

  const renderTransactionAnimation = () => {
    if (transactionStep === 'success' && showSuccess) {
      return (
        <SuccessAnimation>
          <div className="success-icon">
            <IconCheck size={48} />
          </div>
          <Text size="lg" fw={600} mb="xs">Transaction Successful!</Text>
          <Text size="sm" c="dimmed">
            Your {type} transaction has been completed successfully.
          </Text>
        </SuccessAnimation>
      );
    }

    if (transactionStep === 'error') {
      return (
        <ErrorAnimation>
          <div className="error-icon">
            <IconX size={48} />
          </div>
          <Text size="lg" fw={600} mb="xs">Transaction Failed</Text>
          <Text size="sm" c="dimmed" mb="lg">
            {error || 'An error occurred while processing your transaction.'}
          </Text>
          <Button onClick={() => {
            setTransactionStep('idle');
            setError(null);
          }}>
            Try Again
          </Button>
        </ErrorAnimation>
      );
    }

    if (loading && ['preparing', 'executing'].includes(transactionStep)) {
      return (
        <LoadingAnimation>
          <Text size="lg" fw={600} mb="md">Processing Transaction</Text>
          <TransactionSteps>
            <StepItem 
              active={transactionStep === 'preparing'} 
              completed={['executing', 'success'].includes(transactionStep)}
            >
              <div className="step-icon">
                {getStepIcon('preparing', transactionStep)}
              </div>
              <div className="step-text">Preparing transaction...</div>
            </StepItem>
            <StepItem 
              active={transactionStep === 'executing'} 
              completed={transactionStep === 'success'}
            >
              <div className="step-icon">
                {getStepIcon('executing', transactionStep)}
              </div>
              <div className="step-text">Executing on blockchain...</div>
            </StepItem>
            <StepItem active={transactionStep === 'success'}>
              <div className="step-icon">
                {getStepIcon('success', transactionStep)}
              </div>
              <div className="step-text">Transaction complete</div>
            </StepItem>
          </TransactionSteps>
        </LoadingAnimation>
      );
    }

    return null;
  };

  if (!currentWallet) {
    return (
      <Modal opened={opened} onClose={onClose} title="Error" size="md" centered>
        <Text>You must select a wallet to perform transactions.</Text>
        <Group justify="flex-end" mt="md">
          <Button onClick={onClose}>Close</Button>
        </Group>
      </Modal>
    );
  }

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title={!loading && transactionStep === 'idle' ? transactConfig.title : ''}
      size="md"
      centered
      closeOnClickOutside={!loading}
      closeOnEscape={!loading}
    >
      <TransactContainer>
        {renderTransactionAnimation()}
        
        {transactionStep === 'idle' && (
          <>
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

          {transactConfig.fields.sender && (
            <TextInput
              label="Sender Address"
              placeholder="Enter sender address"
              value={formData.sender}
              onChange={(event) => setFormData({ ...formData, sender: event.target.value })}
              mb="md"
              required
            />
          )}

          {transactConfig.fields.amount && (
            <NumberInput
              label={`Amount${selectedToken ? ` (${selectedToken.symbol})` : ''}`}
              placeholder="0.0"
              value={formData.amount}
              onChange={(value) => setFormData({ ...formData, amount: value || 0 })}
              mb="md"
              min={0}
              step={selectedToken ? 1 / Math.pow(10, selectedToken.decimals) : 0.000000001}
              precision={selectedToken?.decimals || 9}
              required
              description={selectedToken ? `Decimals: ${selectedToken.decimals}` : 'Decimals: 9'}
            />
          )}

          {transactConfig.fields.token && (
            <Select
              label="Token"
              placeholder="Select token"
              value={formData.token}
              onChange={(value) => setFormData({ ...formData, token: value || 'PSY' })}
              data={tokens.map(token => ({
                value: token.symbol,
                label: `${token.name} (${token.symbol})`,
              }))}
              mb="md"
              required
            />
          )}


              <Group justify="flex-end" gap="sm">
                <Button variant="outline" onClick={handleClose} disabled={loading}>
                  Cancel
                </Button>
                <Button 
                  onClick={handleSubmit}
                  disabled={
                    loading ||
                    (transactConfig.fields.recipient && !formData.recipient) ||
                    (transactConfig.fields.sender && !formData.sender) ||
                    (transactConfig.fields.amount && formData.amount <= 0)
                  }
                >
                  {loading ? 'Processing...' : transactConfig.confirmText}
                </Button>
              </Group>
            </TransactForm>
          </>
        )}
      </TransactContainer>
    </Modal>
  );
};

export default TransactModal;