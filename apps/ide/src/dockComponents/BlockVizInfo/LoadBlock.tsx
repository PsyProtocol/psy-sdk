import { useState } from 'react';
import styles from './BlockVizInfo.module.scss';
import { Fieldset, NumberInput, Button, Group } from '@mantine/core';
import { ICitySynthBlockConfig, ICitySynthBlockResult, synthPlanner } from '@qstudio/city-block';

interface ILoadBlockProps {
  onLoadBlock: (checkpointId: number) => Promise<any>;
}
function num(x: number | string): number {
  return typeof x === 'string' ? parseInt(x) : x;
}

const LoadBlock: React.FC<ILoadBlockProps> = ({ onLoadBlock }) => {
  const [checkpointId, setCheckpointId] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);

  return (
    <div className={styles.bpContent}>
      <Fieldset legend="Load Block from RPC">
        <NumberInput
          size="xs"
          radius="xs"
          label="Checkpoint ID"
          description="The Checkpoint/Block Number to Load"
          placeholder="Checkpoint ID/Block Number..."
          value={checkpointId}
          onChange={(value) => setCheckpointId(num(value))}
        />
        <Group justify="flex-end" mt="md"><Button onClick={() => {
          setLoading(true);
          onLoadBlock(checkpointId).then(() => {
            setLoading(false);
          }).catch((e) => {
            setLoading(false);
            console.error("ERROR: ", e);
          });
        }} loading={loading}>Load Block</Button>
        </Group>
      </Fieldset>

    </div>

  )
};

export default LoadBlock;