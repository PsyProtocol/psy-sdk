import { useEffect, useRef } from 'react';
import styles from './BlockViz.module.scss';
import QVizRenderer from '../../qviz/QVizRenderer';
import { ITreeJunctionLayout, QSceneManager, RectSide } from '@qstudio/core';
import {CityBlockSceneManager, EXAMPLE_SCENARIO_2} from '@qstudio/qviz-city';
import { setupWidgetStyles } from '../../qviz/widgetStyles';
import { ISimpleCityBlock } from '@qstudio/city-block';
interface IWidgetInfo {
  type: string;
  id: string;
  config: string;
}
interface IBlockVizProps {
  addResizeEventListener: (cb: ()=>void)=>void;
  removeResizeEventListener: (cb: ()=>void)=>void;
  onSelectWidget?: (widgetInfo: IWidgetInfo | null)=>void;
  scenario: ISimpleCityBlock;
}
const treeJunctionLayout: ITreeJunctionLayout = {
  direction: RectSide.Bottom,
  siblingSpacing: 20,
  levelSpacing: 20,
  parentAnchor: RectSide.Bottom,
  childAnchor: RectSide.Top,
  edgeClassName: styles.simpleEdge,
};
const renderWithPaper = (mgr: CityBlockSceneManager)=>{
  mgr.qscene.vizPaper.clear();


  const treeWidget = mgr.getRootWidget();
  
  mgr.qscene.addWidget(treeWidget);
  const swRoot = document.createElementNS("http://www.w3.org/2000/svg", "g");
  mgr.qscene.vizPaper.root.appendChild(swRoot);

  treeWidget.render(mgr.qscene.getRenderContext(), swRoot);
  treeWidget.layout();
  treeWidget.render(mgr.qscene.getRenderContext(), swRoot);
  /*
  swRoot.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 12, fill: "red"}));
  swRoot.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 8, fill: "black"}));*/
  



};
function resolveWidgetInfo(element: any){
  let cur: any = element;
  while(cur.tagName.toUpperCase() !== "SVG"){
    if(cur.dataset.qWidgetType&& cur.dataset.qWidgetId){
      return {type: cur.dataset.qWidgetType, id: cur.dataset.qWidgetId, config: cur.dataset.qWidgetConfig||""};
    }else if(cur.parentElement){
      cur = cur.parentElement;
    }else{
      break;
    }
  }
  return null;
}
const BlockVizComponent: React.FC<IBlockVizProps> = ({ addResizeEventListener, removeResizeEventListener, onSelectWidget, scenario }) => {
  const sceneManager = useRef<CityBlockSceneManager>();

  useEffect(()=>{
    if(sceneManager.current){
      const onResizeEv = ()=>{
        if(sceneManager.current) {
          sceneManager.current.qscene.forceResize();
        }
      };

      const onClickEv = (ev: MouseEvent)=>{
        const widgetInfo = resolveWidgetInfo(ev.target);
        if(typeof onSelectWidget === "function" && widgetInfo){
          onSelectWidget(widgetInfo);
        }
      };
      const onDoubleClickEv = (ev: MouseEvent)=>{
        const widgetInfo = resolveWidgetInfo(ev.target);
        if(typeof onSelectWidget === "function"){
          onSelectWidget(widgetInfo);
        }
      };
      const svg = sceneManager.current.qscene.vizPaper.svg.node;
      svg.addEventListener("click", onClickEv);
      svg.addEventListener("dblclick", onDoubleClickEv);

      addResizeEventListener(onResizeEv);
      return ()=>{
        removeResizeEventListener(onResizeEv);
        svg.removeEventListener("click", onClickEv);
        svg.removeEventListener("dblclick", onDoubleClickEv);
      };
    }
  },[sceneManager, addResizeEventListener, removeResizeEventListener, onSelectWidget]);

  useEffect(()=>{
    if(sceneManager.current){
      sceneManager.current.updateScenario(scenario);
      renderWithPaper(sceneManager.current);
    }
  },[scenario])
  return (
    <div className={styles.stageDockPage}>
      <QVizRenderer
        onRendererManager={(vp)=>{
          if(vp){
            if(!sceneManager.current){
              console.log("vp: ",vp);
              const sm = new QSceneManager(vp);
              setupWidgetStyles(sm.styleResolver);
              const sm2 = new CityBlockSceneManager(sm, scenario);

              sceneManager.current = sm2;
              renderWithPaper(sm2);
            }else{
              sceneManager.current.setVizPaper(vp);
              renderWithPaper(sceneManager.current);
            }
          }

        }}
      />
      <div className={styles.stageControls}>
        <button className={styles.scButton} onClick={()=>{
          if(sceneManager.current){
            sceneManager.current.proveNextJobs(sceneManager.current.qscene.getRenderContext(), 1000).catch(console.error);
          }
        }}>Next</button>
      </div>
    </div>
  )
};

export {
  BlockVizComponent,
};

export type {
  IWidgetInfo,
};