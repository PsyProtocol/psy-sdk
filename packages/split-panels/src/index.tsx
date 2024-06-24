import React, { useEffect } from 'react';
import { Layout, Model, TabNode, IJsonModel, IIcons, Actions, Node } from '@qstudio/flex-layout';
import { EventHub } from '@qstudio/utils';

import { SplitPanelsEvent, SplitPanelsEventType } from '@qstudio/eventhubs'
import { ComponentRenderer, IBaseConfig, IBaseContext, useSplitPanels } from './hooks/useSplitPanels';
interface ISplitPanelsProps<C extends { splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent> }> {
  model: Model;
  editorContext: C;
  resolveTabIcon: (node: TabNode) => React.ReactNode;
  resolveTab: (node: TabNode) => React.ReactNode;
}

function SplitPanelsInner<C extends { splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent> }>({ model, resolveTabIcon, resolveTab }: ISplitPanelsProps<C>, ref: React.Ref<Layout>) {
  return (
    <Layout ref={ref} model={model} factory={resolveTab} iconFactory={resolveTabIcon} />
  )
}
const SplitPanels = React.forwardRef(SplitPanelsInner);
function SplitPanelsManaged<P extends string, X extends IBaseContext, C extends IBaseConfig<P, X>>({ modelJson, editorContext, config, onActiveFileChanged, notifyResize }: { modelJson: IJsonModel, editorContext: X, config: C, onActiveFileChanged: (path: string) => void, notifyResize: () => any }): JSX.Element {
  const { ref, model, resolveTab, resolveTabIcon } = useSplitPanels(modelJson, editorContext, config, notifyResize);
  //const setActiveFile = useActiveFile(s => s.setActiveFile);
  useEffect(()=>{
    window.addEventListener('resize', notifyResize);
    return () => {
      window.removeEventListener('resize', notifyResize);
    }
  });
  (window as any)._notifyResize =  notifyResize;
  return <Layout ref={ref} model={model} factory={resolveTab} iconFactory={resolveTabIcon}
    onAction={(action) => {
      if(action.type === Actions.ADJUST_BORDER_SPLIT || action.type === Actions.ADJUST_SPLIT || action.type === Actions.MOVE_NODE|| action.type === Actions.SELECT_TAB){
        editorContext.splitPanelsEventHub.notify(SplitPanelsEventType.ResizePanels, {});
      }
      console.log("action",action);
      setTimeout(notifyResize, 150);
      if (action.type === Actions.SELECT_TAB && action.data && action.data.tabNode) {

        const d = model.getNodeById(action.data.tabNode)
        if (d) {
          const cfg = (d as any).getConfig();
          if (cfg && cfg.filePath) {
            notifyResize();
            onActiveFileChanged(cfg.filePath);
          }
        }
      } else if (action.type === Actions.ADD_NODE && action.data && action.data.json) {
        if (action.data.json.type === "tab" && action.data.json.config && action.data.json.config.filePath) {
          onActiveFileChanged(action.data.json.config.filePath);
        }
      } else if (action.type === Actions.SET_ACTIVE_TABSET && action.data && action.data.tabsetNode) {
        setTimeout(() => {
          const active = model.getActiveTabset();
          if (active) {
            const n = active.getSelectedNode();
            if (n && (n as any).getConfig) {
              const cfg = (n as any).getConfig();
              if (cfg && cfg.filePath) {
                onActiveFileChanged(cfg.filePath);
              }

            }

          }

        }, 50);
      }
      return action;
    }} />;
}

export type {
  IBaseConfig,
  IBaseContext,
  ComponentRenderer,
}
export {
  SplitPanels,
  SplitPanelsManaged,
}