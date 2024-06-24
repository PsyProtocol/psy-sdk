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
};
function simplePair(name: string){

  const treeRootNode = new QWRect({width: 50, height: 50, borderWidth: 2}, {x: 0, y: 0});
  treeRootNode.updateState({label: name, color: "blue"});
  

  const children = seq(2).map(x=>{
    const child = new QWRect({width: 50, height: 50, borderWidth: 2}, {x: 0, y: 0});
    child.updateState({label: "Child "+x, color: "#a55"});
    return child;
  });

  const treeJunction = QWTreeJunction.create(treeRootNode, children, {layout: treeJunctionLayout});
  return treeJunction;
}


function simplePairProofs(name: string){

  const treeRootNode = new QWProof({}, {x: 0, y: 0});
  treeRootNode.updateState({label: name});
  

  const children = seq(2).map(x=>{
    const treeRootNode = new QWProof({}, {x: 0, y: 0});
    treeRootNode.updateState({label: "Agg "+x});
    return treeRootNode;
  });

  const treeJunction = QWTreeJunction.create(treeRootNode, children, {layout: treeJunctionLayout});
  return treeJunction;
}
function simpleScene(mgr: QSceneManager, rootName = "Root"){
  let left = simplePairProofs("Left");
  let right = simplePairProofs("Right");

  const treeRootNode = new QWProof({}, {x: 0, y: 0});
  treeRootNode.updateState({label: rootName});
  
  
  const treeJunction = QWTreeJunction.create(treeRootNode, [left, right], {layout: treeJunctionLayout});
  return treeJunction;
}
function simpleScene2(mgr: QSceneManager){
  let left = simpleScene(mgr, "LC");
  let right = simpleScene(mgr, "RC");

  const treeRootNode = new QWProof({}, {x: 0, y: 0});
  treeRootNode.updateState({label: "RT"});
  
  const treeJunction = QWTreeJunction.create(treeRootNode, [left, right], {layout: treeJunctionLayout});
  return treeJunction;
}
const renderWithPaper = (mgr: QSceneManager)=>{
  mgr.vizPaper.clear();

  /*
  mgr.vizPaper.root.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 12, fill: "red"}));
  mgr.vizPaper.root.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 8, fill: "black"}));
  


  const treeRootNode = new QWRect({width: 200, height: 50, borderWidth: 2}, {x: 0, y: 0});
  treeRootNode.updateState({label: "Hello World", color: "blue"});
  

  const children = seq(2).map(x=>{
    const child = new QWRect({width: 100, height: 100, borderWidth: 2}, {x: 0, y: 0});
    child.updateState({label: "Child "+x, color: "#a55"});
    return child;
  })
  
  const treeJunction = QWTreeJunction.create(treeRootNode, children, {layout: treeJunctionLayout});
  mgr.addWidget(treeJunction);
  const swRoot = document.createElementNS("http://www.w3.org/2000/svg", "g");
  mgr.vizPaper.root.appendChild(swRoot);

  treeJunction.render(mgr.getRenderContext(), swRoot);
  treeJunction.layout();
  treeJunction.render(mgr.getRenderContext(), swRoot);
  */

  const treeWidget = simpleScene2(mgr);
  
  mgr.addWidget(treeWidget);
  const swRoot = document.createElementNS("http://www.w3.org/2000/svg", "g");
  mgr.vizPaper.root.appendChild(swRoot);

  treeWidget.render(mgr.getRenderContext(), swRoot);
  treeWidget.layout();
  treeWidget.render(mgr.getRenderContext(), swRoot);
  swRoot.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 12, fill: "red"}));
  swRoot.appendChild(makeSVGElemAttributes("circle", {cx: 0, cy: 0, r: 8, fill: "black"}));
  



};
const StageDockComponent: React.FC<IStageDockComponentProps> = ({ filePath, fileEventHub, fileStore, ctx }) => {

  const originId = useMemo<string>(() => uuidv4(), []);
  const realFilePath = useRenamableFile(filePath, fileEventHub);
  const sceneManager = useRef<QSceneManager>();

  useEffect(()=>{
    if(sceneManager.current){
      const onResizeEv = ()=>{
        if(sceneManager.current) {
          sceneManager.current.forceResize();
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
            console.log("vp: ",vp);
            const sm = new QSceneManager(vp);
            sceneManager.current = sm;
            renderWithPaper(sm);
          }

        }}
      />
      <div className={styles.stageControls}>
        <button className={styles.scButton} onClick={()=>{

        }}>Next</button>
      </div>
    </div>
  )
};

export default StageDockComponent;