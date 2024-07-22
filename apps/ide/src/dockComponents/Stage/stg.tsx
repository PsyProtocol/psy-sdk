import { useEffect, useMemo, useRef, useState } from 'react';
import { EventHub, seq, uuidv4 } from '@qstudio/utils';
import { useRenamableFile } from '../../hooks/useRenamableFile';
import styles from './Stage.module.scss';
import { EditorUIEventType, IDEMenuId, ProjectFilesEvent, ProjectFilesEventType, SplitPanelsEventType } from '@qstudio/eventhubs';
import { ISyncFileStore } from '@qstudio/storage';
import { IDEContext } from '../../utils/ideContext';
import { getFileExtForFileName, getLanguageForFilePath } from '../../utils/fileIcons';
import QVizRenderer from '../../qviz/QVizRenderer';
import {makeElemAttributes, makeSVGElemAttributes} from '@qstudio/qsvg';
import { ITreeJunctionLayout, NodeAnchor, QSceneManager, QWProof, QWRect, QWTreeJunction, RectSide, simpleDebugTree } from '@qstudio/core';
import {CityBlockSceneManager, EXAMPLE_SCENARIO, EXAMPLE_SCENARIO_2} from '@qstudio/qviz-city';
import { setupWidgetStyles } from '../../qviz/widgetStyles';
interface IStageDockComponentProps {
  filePath: string;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  fileStore: ISyncFileStore;
  ctx: IDEContext;
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
const StageDockComponent: React.FC<IStageDockComponentProps> = ({ filePath, fileEventHub, fileStore, ctx }) => {

  const originId = useMemo<string>(() => uuidv4(), []);
  const realFilePath = useRenamableFile(filePath, fileEventHub);
  const sceneManager = useRef<CityBlockSceneManager>();

  useEffect(()=>{
    if(sceneManager.current){
      const onResizeEv = ()=>{
        if(sceneManager.current) {
          sceneManager.current.qscene.forceResize();
        }
      };
      ctx.splitPanelsEventHub.on(SplitPanelsEventType.ResizePanels, onResizeEv);

      window.addEventListener("resize", onResizeEv);
      return ()=>{
        window.removeEventListener("resize", onResizeEv);
        ctx.splitPanelsEventHub.remove(SplitPanelsEventType.ResizePanels, onResizeEv);
      };
    }
  },[sceneManager, ctx]);
  return (
    <div className={styles.stageDockPage}>
      <QVizRenderer
        onRendererManager={(vp)=>{
          if(vp){
            if(!sceneManager.current){
              console.log("vp: ",vp);
              const sm = new QSceneManager(vp);
              setupWidgetStyles(sm.styleResolver);
              const sm2 = new CityBlockSceneManager(sm, EXAMPLE_SCENARIO_2);

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

export default StageDockComponent;