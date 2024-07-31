import { ISimpleCityBlock, PSCityBlock } from "@qstudio/city-block";

enum BlockVizEventType {
  SetBlockScenario = 0,
  SelectVizWidget = 1,
}

enum VizWidgetType {
  None = "None",
  QWCityProof = "QWCityProof",
}


interface IBlockVizEventBase {
  type: BlockVizEventType;
}

// message types

interface IBlockVizSetBlockScenarioEvent extends IBlockVizEventBase {
  type: BlockVizEventType.SetBlockScenario;
  scenario: ISimpleCityBlock;
  psBlock?: PSCityBlock<any>;
}

interface IBlockVizSelectVizWidgetEvent extends IBlockVizEventBase {
  type: BlockVizEventType.SelectVizWidget;
  widgetType: VizWidgetType;
  widgetId: string;
  altId: string;
}

type IBlockVizEvent = IBlockVizSetBlockScenarioEvent | IBlockVizSelectVizWidgetEvent;

export {
  BlockVizEventType,
  VizWidgetType,
}


export type {
  IBlockVizEvent,
  IBlockVizSetBlockScenarioEvent,
  IBlockVizSelectVizWidgetEvent,
}
