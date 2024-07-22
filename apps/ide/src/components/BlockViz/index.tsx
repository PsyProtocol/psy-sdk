import { useCallback, useEffect, useState } from "react";
import { IDEContext } from "../../utils/ideContext";
import { BlockVizComponent, IWidgetInfo } from "./BlockViz";
import { BlockVizEventType, SplitPanelsEventType, VizWidgetType } from "@qstudio/eventhubs";

interface IBlockVizDockPageProps {
  ctx: IDEContext;
}

function tryParseWidgetConfig(widgetConfig: string): any {
  try {
    return JSON.parse(widgetConfig);
  } catch (e) {
    return null;
  }
}
const BlockVizDockComponent: React.FC<IBlockVizDockPageProps> = ({ ctx }) => {

  const [scenario, setScenario] = useState(ctx.blockVizDataStore.blockScenario);
  const addResizeEventListener = useCallback(
    (cb: () => void) => {
      ctx.splitPanelsEventHub.on(SplitPanelsEventType.ResizePanels, cb);
      window.addEventListener("resize", cb);
    },
    [ctx.splitPanelsEventHub]
  );
  const removeResizeEventListener = useCallback(
    (cb: () => void) => {
      ctx.splitPanelsEventHub.remove(SplitPanelsEventType.ResizePanels, cb);
      window.removeEventListener("resize", cb);
    },
    [ctx.splitPanelsEventHub]
  );

  const onSelectWidget = useCallback((widgetInfo: IWidgetInfo | null) => {
    console.log("Selected Widget: ", widgetInfo);
    if(widgetInfo){
      const config = tryParseWidgetConfig(widgetInfo.config||"{}")||{};
      if(widgetInfo.type === "QWCityProof" && config.jobId){
        ctx.blockVizEventHub.notify({
          type: BlockVizEventType.SelectVizWidget,
          widgetId: widgetInfo.id,
          widgetType: VizWidgetType.QWCityProof,
          altId: config.jobId,
        })
      }
    }else{
      ctx.blockVizEventHub.notify({
        type: BlockVizEventType.SelectVizWidget,
        widgetId: "",
        widgetType: VizWidgetType.None,
        altId: "",
      })
    }
  },[ctx.blockVizEventHub]);

  useEffect(()=>{
    ctx.blockVizEventHub.on(BlockVizEventType.SetBlockScenario, (scenario)=>{
      console.log("block scenario", scenario);
      setScenario(scenario.scenario);
    });

  },[ctx.blockVizEventHub])

  return (
    <BlockVizComponent
      addResizeEventListener={addResizeEventListener}
      removeResizeEventListener={removeResizeEventListener}
      onSelectWidget={onSelectWidget}
      scenario={scenario}
    />
  );
};

export default BlockVizDockComponent;
