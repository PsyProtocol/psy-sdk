import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import styles from './BlockVizInfo.module.scss';
import { IDEContext } from '../../utils/ideContext';
import { Fieldset, NumberInput, Textarea, Button, Group, Tabs } from '@mantine/core';
import { depSerializedToProofNodes, ICitySynthBlockConfig, ICitySynthBlockResult, PSCityBlock, synthPlanner } from '@qstudio/city-block';
import { genCodeForSVG } from '@qstudio/qsvg';
import copy from 'copy-to-clipboard';
import { FloatingIndicatorMenu } from '../../components/FloatingIndicatorMenu';
import { ISimpleCityBlock } from '@qstudio/city-block';
import { getProofStoreConfigForCheckpoint } from '@qstudio/city-block';
import SynthBlock from './SynthBlock';
import LoadBlock from './LoadBlock';
import { BlockVizInfoPane } from '../../components/BlockVizInfoPane/BlockVizInfoPane';
import { IRealBlockVizJobInfo } from '../../components/BlockVizInfoPane/content/types';
import { getCircuitContentInfo } from '../../components/BlockVizInfoPane/content/circuits';

interface IBlockVizInfoDockComponentProps {
  ctx: IDEContext;
}
const TabOptions = [
  { label: "Info", value: "info" },
  { label: "Custom Block", value: "custom_block" },
  { label: "Load Block", value: "load_block" },
];


const SynthBlockTab: React.FC<IBlockVizInfoDockComponentProps> = ({ ctx }) => {
  const onSetSynthBlock = useCallback((b: ICitySynthBlockResult)=>{
    const simple: ISimpleCityBlock = {
      stateTransitionRoot: depSerializedToProofNodes(b.root_state_transition),
      sighashProofs: b.sighash_proofs,
    };
    ctx.blockVizDataStore.setBlockScenario(simple);
  },[ctx.blockVizDataStore]);
  return (
    <div className={styles.synthBlockTab}>
      <LoadBlock onLoadBlock={async (checkpointId)=>{
        const result = await getProofStoreConfigForCheckpoint(ctx.rpc, checkpointId, 0);

        const synthBlock = synthPlanner(result);

        const simple: ISimpleCityBlock = {
          stateTransitionRoot: depSerializedToProofNodes(synthBlock.root_state_transition),
          sighashProofs: synthBlock.sighash_proofs,
        };
        const psBlock = new PSCityBlock<IRealBlockVizJobInfo>(ctx.rpc, ctx.dogeRPC, synthBlock, result, simple, (ctx, jobIdHex)=>getCircuitContentInfo(ctx, jobIdHex));

        ctx.blockVizDataStore.setBlockScenario(simple, psBlock);


      }} />
    <SynthBlock onSetSynthBlock={onSetSynthBlock} />
    </div>
  );
};

const InfoBlockTab: React.FC<IBlockVizInfoDockComponentProps> = ({ ctx }) => {
  return (
    <BlockVizInfoPane blockVizDataStore={ctx.blockVizDataStore} />
  );
};
const BlockVizInfoDockComponent: React.FC<IBlockVizInfoDockComponentProps> = ({ ctx }) => {
  const [activeTab, setActiveTab] = useState<any>("info");
  
  return (
    <div className={styles.blockVizInfoDockPage}>

    <Tabs value={activeTab} onChange={setActiveTab} radius="xs">
      <Tabs.List grow>
        <Tabs.Tab value="info">Widget Info</Tabs.Tab>
        <Tabs.Tab value="custom_block">Load Block</Tabs.Tab>
      </Tabs.List>
    </Tabs>
      <div className={styles.bpContent}>
      {activeTab === "info" && <InfoBlockTab ctx={ctx} />}
      {activeTab === "custom_block" && <SynthBlockTab ctx={ctx} />}
      </div>
  </div>
  )
};

export default BlockVizInfoDockComponent;