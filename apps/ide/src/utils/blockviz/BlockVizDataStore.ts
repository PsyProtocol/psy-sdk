import { EXAMPLE_SCENARIO_2 } from "@qstudio/qviz-city";
import { DogeLinkElectrsComboRPC } from "doge-sdk";
import { CityRPCProvider } from "@qstudio/city-sdk";
import { EventHub } from "@qstudio/utils";
import { BlockVizEventType, IBlockVizEvent, IBlockVizSelectVizWidgetEvent, IBlockVizSetBlockScenarioEvent, VizWidgetType } from "@qstudio/eventhubs";
import { deserializeJobId, ISimpleCityBlock, PSCityBlock } from "@qstudio/city-block";
import { IRealBlockVizJobInfo } from "../../components/BlockVizInfoPane/content/types";

class BlockVizDataStore {
  checkpointId: number = -1;
  blockScenario: ISimpleCityBlock = EXAMPLE_SCENARIO_2;
  dogeRPC: DogeLinkElectrsComboRPC;
  rpc: CityRPCProvider;
  blockVizEventHub: EventHub<BlockVizEventType, IBlockVizEvent>;
  psBlock?: PSCityBlock<IRealBlockVizJobInfo>;
  selectedJobId: string = "";
  constructor(dogeRPC: DogeLinkElectrsComboRPC, rpc: CityRPCProvider, blockVizEventHub: EventHub<BlockVizEventType, IBlockVizEvent>) {
    this.dogeRPC = dogeRPC;
    this.rpc = rpc;
    this.blockVizEventHub = blockVizEventHub;
    this.onBlockScenarioChange = this.onBlockScenarioChange.bind(this);
    this.setupHandlers();
  }

  setBlockScenario(scenario: ISimpleCityBlock, psBlock?: PSCityBlock<IRealBlockVizJobInfo>){
    this.blockScenario = scenario;
    this.psBlock = psBlock;
    console.log("psblock",psBlock);
    this.checkpointId = deserializeJobId(scenario.stateTransitionRoot.id).goal_id;
    this.blockVizEventHub.notify(BlockVizEventType.SetBlockScenario, {scenario, psBlock});
  }

  onSelectedJobIdChange(ev: IBlockVizSelectVizWidgetEvent){
    if(ev.widgetType === VizWidgetType.QWCityProof){
      this.selectedJobId = ev.altId||"";
    }else{
      this.selectedJobId = "";
    }
  }

  onBlockScenarioChange(ev: IBlockVizSetBlockScenarioEvent){
    this.blockScenario = ev.scenario;
    this.psBlock = ev.psBlock;
  }

  setupHandlers() {
    this.blockVizEventHub.on(BlockVizEventType.SetBlockScenario, this.onBlockScenarioChange);
  }

  dispose() {
    this.blockVizEventHub.remove(BlockVizEventType.SetBlockScenario, this.onBlockScenarioChange);
  }

}

export {BlockVizDataStore};