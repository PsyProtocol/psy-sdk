import { useState, useEffect } from 'react';
import { EventHub } from '@qstudio/utils';
import { ProjectFilesEvent, ProjectFilesEventType, IFileRenamedEvent, BlockVizEventType, IBlockVizSetBlockScenarioEvent, IBlockVizEvent, IBlockVizSelectVizWidgetEvent, VizWidgetType } from '@qstudio/eventhubs';
import { BlockVizDataStore } from '../utils/blockviz/BlockVizDataStore';
import { ISimpleCityBlock, PSCityBlock } from '@qstudio/city-block';
import { IRealBlockVizJobInfo } from '../components/BlockVizInfoPane/content/types';

export function useVizScenario( blockVizDataStore: BlockVizDataStore) {

  const [scenario, setScenario] = useState<ISimpleCityBlock>(blockVizDataStore.blockScenario);
  const [psBlock, setPSBlock] = useState<PSCityBlock<IRealBlockVizJobInfo>>();
  useEffect(() => {
    const onBlock =  (ev: IBlockVizSetBlockScenarioEvent) => {
      setScenario(ev.scenario);
      setPSBlock(ev.psBlock);
    };
    blockVizDataStore.blockVizEventHub.on(BlockVizEventType.SetBlockScenario, onBlock);
    return () => {
      blockVizDataStore.blockVizEventHub.removeEventListener(BlockVizEventType.SetBlockScenario, onBlock);
    };
  }, [blockVizDataStore.blockVizEventHub]);
  return {scenario, psBlock};
}



export function useSelectedJobId ( blockVizDataStore: BlockVizDataStore) {

  const [selectedJobId, setSelectedJobId] = useState<string>(blockVizDataStore.selectedJobId);
  useEffect(() => {
    const onSelectWidget =  (ev: IBlockVizSelectVizWidgetEvent) => {
      if(ev.widgetType === VizWidgetType.QWCityProof){
        setSelectedJobId(ev.altId||"");
      }else{
        setSelectedJobId("");
      }
    };
    blockVizDataStore.blockVizEventHub.on(BlockVizEventType.SelectVizWidget, onSelectWidget);
    return () => {
      blockVizDataStore.blockVizEventHub.removeEventListener(BlockVizEventType.SelectVizWidget, onSelectWidget);
    };
  }, [blockVizDataStore.blockVizEventHub]);
  return selectedJobId;
}


export function useSelectedJobIdAndVizScenario( blockVizDataStore: BlockVizDataStore) {

  const [{scenario,psBlock}, setScenario] = useState<{scenario: ISimpleCityBlock, psBlock?: PSCityBlock<IRealBlockVizJobInfo>}>({scenario: blockVizDataStore.blockScenario, psBlock: blockVizDataStore.psBlock});
  const [selectedJobId, setSelectedJobId] = useState<string>(blockVizDataStore.selectedJobId);
//
  useEffect(() => {
    const onBlock =  (ev: IBlockVizSetBlockScenarioEvent) => {
      setScenario({scenario: ev.scenario, psBlock: ev.psBlock});
    };
    const onSelectWidget =  (ev: IBlockVizSelectVizWidgetEvent) => {
      if(ev.widgetType === VizWidgetType.QWCityProof){
        setSelectedJobId(ev.altId||"");
      }else{
        setSelectedJobId("");
      }
    };
    blockVizDataStore.blockVizEventHub.on(BlockVizEventType.SetBlockScenario, onBlock);

    blockVizDataStore.blockVizEventHub.on(BlockVizEventType.SelectVizWidget, onSelectWidget);
    return () => {
      blockVizDataStore.blockVizEventHub.removeEventListener(BlockVizEventType.SelectVizWidget, onSelectWidget);
      blockVizDataStore.blockVizEventHub.removeEventListener(BlockVizEventType.SetBlockScenario, onBlock);
    };
  }, [blockVizDataStore.blockVizEventHub]);
  return {selectedJobId, scenario, psBlock};
}
