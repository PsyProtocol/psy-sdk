import {  useState } from 'react';
import styles from './BlockVizInfo.module.scss';
import { Fieldset, NumberInput, Button, Group } from '@mantine/core';
import { ICitySynthBlockConfig, ICitySynthBlockResult, synthPlanner } from '@qstudio/city-block';

interface IBlockPlannerDockComponentProps {
  onSetSynthBlock: (block: ICitySynthBlockResult) => void;
}
function num(x: number | string): number {
  return typeof x === 'string' ? parseInt(x) : x;
}

const SynthBlock: React.FC<IBlockPlannerDockComponentProps> = ({ onSetSynthBlock }) => {
  const [config, setConfig] = useState<ICitySynthBlockConfig>({
    checkpoint_id: 0,
    job_config: {
      register_user_count: 0,
      claim_deposit_count: 0,
      token_transfer_count: 0,
      add_withdrawal_count: 0,
      process_withdrawal_count: 0,
      add_deposit_count: 0
    }
  });
  const onGenerateBlock = () => {
    const result = synthPlanner(config);
    onSetSynthBlock(result);
  };
  return (
      <div className={styles.bpContent}>
    <Fieldset legend="Block Planner">
    <NumberInput
      size="xs"
      radius="xs"
      label="Register Users"
      description="Number of Register User transactions to include in the block"
      placeholder="# of new users..."
      value={config.job_config.register_user_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, register_user_count: num(value) } })}
    />
    <NumberInput
      size="xs"
      radius="xs"
      label="Token Transfers"
      description="Number of Token Transfer transactions to include in the block"
      placeholder="# of token transfers..."
      value={config.job_config.token_transfer_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, token_transfer_count: num(value) } })}
    />
    <NumberInput
      size="xs"
      radius="xs"
      label="Deposits"
      description="Number of layer 1 deposits to be included in the block"
      placeholder="# of deposits..."
      value={config.job_config.add_deposit_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, add_deposit_count: num(value) } })}
    />
    <NumberInput
      size="xs"
      radius="xs"
      label="Claimed Deposits"
      description="Number of layer 1 deposits claimed by users in the block"
      placeholder="# of claimed deposits..."
      value={config.job_config.claim_deposit_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, claim_deposit_count: num(value) } })}
    />
    <NumberInput
      size="xs"
      radius="xs"
      label="Requested Withdrawals"
      description="Number of withdrawals requested by users to transfer funds from L2 back to L1"
      placeholder="# of requested withdrawals..."
      value={config.job_config.add_withdrawal_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, add_withdrawal_count: num(value) } })}
    />
    <NumberInput
      size="xs"
      radius="xs"
      label="Processed Withdrawals"
      description="Number of withdrawals to be processed in the current block (send funds from L2 back to L1)"
      placeholder="# of processed withdrawals..."
      value={config.job_config.process_withdrawal_count}
      onChange={(value) => setConfig({ ...config, job_config: { ...config.job_config, process_withdrawal_count: num(value) } })}
    />
      <Group justify="flex-end" mt="md">
        <Button onClick={onGenerateBlock}>Generate Block</Button>
      </Group>
    </Fieldset>

    </div>
  )
};

export default SynthBlock;