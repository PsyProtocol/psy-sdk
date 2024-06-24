import { useEffect, useMemo, useRef, useState } from 'react';
import styles from './BlockPlanner.module.scss';
import { IDEContext } from '../../utils/ideContext';
import { Fieldset, NumberInput, Textarea, Button, Group } from '@mantine/core';
import { ICitySynthBlockConfig, ICitySynthBlockResult, synthPlanner } from '@qstudio/city-block';
import { genCodeForSVG } from '@qstudio/qsvg';
import copy from 'copy-to-clipboard';

interface IBlockPlannerDockComponentProps {
  ctx: IDEContext;
}
function num(x: number | string): number {
  return typeof x === 'string' ? parseInt(x) : x;
}

const SVGCodeGenHelper: React.FC = () => {
  let [value, setValue] = useState<string>('');
  let [result, setResult] = useState<string>('');
  return (
    <div>
      <Textarea
        value={value}
        onChange={(e) => setValue(e.currentTarget.value)}
        placeholder="Enter SVG code here..."
      />
      <Button onClick={() => {
        const result = (genCodeForSVG(value));
        copy(result);
        setResult(result);
      }}>Parse</Button>
      <Textarea
        value={result}
        disabled
      />
    </div>
  );
}
const BlockPlannerDockComponent: React.FC<IBlockPlannerDockComponentProps> = ({ ctx }) => {
  const [plannedBlock, setPlannedBlock] = useState<ICitySynthBlockResult>();
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
    setPlannedBlock(result);
  };
  return (
    <div className={styles.blockPlannerDockPage}>
      <div className={styles.bpContent}>
        <SVGCodeGenHelper />
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
    

    {plannedBlock && (
      <div className={styles.resultCon}>

        <Textarea value={JSON.stringify(plannedBlock, null, 2)} disabled />
        </div>
    )}

      </div>
    </div>
  )
};

export default BlockPlannerDockComponent;