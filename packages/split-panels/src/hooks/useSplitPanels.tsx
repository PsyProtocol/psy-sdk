import React from "react";
import { useRef, useEffect, useMemo } from "react";
import {
  EditorLogEventType,
  IEditorLogEvent,
  IFileDeletedEvent,
  IFileRenamedEvent,
  IOpenFileTabEvent,
  IOpenTabEvent,
  ProjectFilesEvent,
  ProjectFilesEventType,
  SplitPanelsEvent,
  SplitPanelsEventType,
} from "@qstudio/eventhubs";
import { EventHub } from "@qstudio/utils";
import {
  Actions,
  IJsonModel,
  Layout,
  Model,
  TabNode,
} from "@qstudio/flex-layout";
import { getAllTabIdsForComponent, getAllTabIdsForFilePath } from "../utils";

interface FileIconBaseProps extends React.SVGAttributes<SVGElement> {
  children?: React.ReactNode;
  size?: string | number;
  color?: string;
  title?: string;
}
type FileIconType = (props: FileIconBaseProps) => JSX.Element;
interface IFileIconDef {
  icon?: FileIconType;
  iconColor?: string;
}

type ComponentRenderer<T extends IBaseContext> = (
  args: any,
  dockCtx: { layout?: Layout; model: Model; nodeId: string; editorContext: T }
) => any;

interface IBaseContext {
  splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>;

}
interface IBaseConfig<P extends string, X extends IBaseContext> {
  editorComponentType: P;
  tabComponentIconMap: Record<P, IFileIconDef>;
  tabComponentTitleMap: Record<P, string>;
  fileExtensionIconMap: Record<string, IFileIconDef>;
  panelComponentMap: Record<P, ComponentRenderer<X>>;
}

function useSplitPanels<
  P extends string,
  X extends IBaseContext,
  C extends IBaseConfig<P, X>
>(modelJson: IJsonModel, editorContext: X, config: C, notifyResize: ()=>any) {
  const layoutRef = useRef<Layout>(null);
  const model = useMemo(() => Model.fromJson(modelJson), [modelJson]);

  useEffect(() => {
    const onOpenTab = (event: IOpenTabEvent) => {
      const tabs = getAllTabIdsForComponent(model.getRoot(), event.tabType);
      if (tabs.length) {
        model.doAction(Actions.selectTab(tabs[0]));
      } else {
        if (layoutRef.current) {
          const active = model.getActiveTabset();
          if (!active) {
            const first = model.getFirstTabSet();
            model.doAction(Actions.setActiveTabset(first.getId()));
          }
          layoutRef.current.addTabToActiveTabSet({
            type: "tab",
            component: event.tabType,
            name:
              config.tabComponentTitleMap[event.tabType as P] || event.tabType,
          });
        }
      }
      notifyResize();
      setTimeout(notifyResize, 100);
    };
    const onOpenFile = (event: IOpenFileTabEvent) => {
      const tabs = getAllTabIdsForFilePath(model.getRoot(), event.filePath);
      if (tabs.length) {
        model.doAction(Actions.selectTab(tabs[0]));
      } else {
        if (layoutRef.current) {
          const active = model.getActiveTabset();
          if (!active) {
            const first = model.getFirstTabSet();
            model.doAction(Actions.setActiveTabset(first.getId()));
          }
          layoutRef.current.addTabToActiveTabSet({
            type: "tab",
            component: config.editorComponentType,

            name: event.filePath.split("/").pop() || event.filePath,
            altName: event.filePath,
            config: {
              filePath: event.filePath,
              ext: event.filePath.split(".").pop() || "",
            },
          });
        }
      }
      notifyResize();
      setTimeout(notifyResize, 100);
    };
    const onRename = (event: IFileRenamedEvent)=>{
      const tabIds = getAllTabIdsForFilePath(model.getRoot(), event.path);
      const fileName = event.newPath.split("/").pop()!;
      tabIds.forEach(id => model.doAction(Actions.renameTab(id, fileName)));
    };
    const onDelete = (event: IFileDeletedEvent) => {
      const tabIds = getAllTabIdsForFilePath(model.getRoot(), event.path);
      tabIds.forEach(id => model.doAction(Actions.deleteTab(id)));
    };
    

    editorContext.fileEventHub.on(ProjectFilesEventType.FileRenamed, onRename);
    editorContext.fileEventHub.on(ProjectFilesEventType.FileDeleted, onDelete);

    editorContext.splitPanelsEventHub.on(
      SplitPanelsEventType.OpenTab,
      onOpenTab
    );
    editorContext.splitPanelsEventHub.on(
      SplitPanelsEventType.OpenFileTab,
      onOpenFile
    );
    return () => {
      editorContext.fileEventHub.remove(ProjectFilesEventType.FileRenamed, onRename);
      editorContext.fileEventHub.remove(ProjectFilesEventType.FileDeleted, onDelete);
      editorContext.splitPanelsEventHub.remove(
        SplitPanelsEventType.OpenTab,
        onOpenTab
      );
      editorContext.splitPanelsEventHub.remove(
        SplitPanelsEventType.OpenFileTab,
        onOpenFile
      );
    };
  }, [model, layoutRef, editorContext, config, editorContext.splitPanelsEventHub, notifyResize]);

  const resolveTabIcon = useMemo(
    () => (node: TabNode) => {
      const component = node.getComponent();
      const tabIconDef = config.tabComponentIconMap[component as P];
      const TabIconBase = tabIconDef?.icon;
      const iconColorBase = tabIconDef?.iconColor;
      if (component === config.editorComponentType) {
        const ext = (node.getConfig() || {}).ext || "";
        if (ext && config.fileExtensionIconMap[ext]) {
          const TabIcon = config.fileExtensionIconMap[ext].icon;
          const iconColor = config.fileExtensionIconMap[ext].iconColor;
          if (TabIcon) {
            return <TabIcon color={iconColor} size={"1em"} className="qedTabIcon"/>;
          }
        }
      }
      if (TabIconBase) {
        return <TabIconBase color={iconColorBase} size={"1em"} className="qedTabIcon"/>;
      } else {
        return <span />;
      }
    },
    [config]
  );
  /*
  const resolveTab = useMemo(
    () => (node: TabNode) => {
      if (layoutRef.current) {
        const component = node.getComponent();
        if (config.panelComponentMap[component as P]) {
          return config.panelComponentMap[component as P](
            node.getConfig() || {},
            {
              layout: layoutRef.current,
              model,
              nodeId: node.getId(),
              editorContext,
            }
          );
        }
      } else {
        return <div></div>;
      }
    },
    [config, layoutRef, model]
  );*/
  const resolveTab =(node: TabNode) => {
    if (layoutRef.current) {
      const component = node.getComponent();
      if (config.panelComponentMap[component as P]) {
        return config.panelComponentMap[component as P](
          node.getConfig() || {},
          {
            layout: layoutRef.current,
            model,
            nodeId: node.getId(),
            editorContext,
          }
        );
      }
    } else {
      return <div></div>;
    }
  };

  return { ref: layoutRef, model, resolveTab, resolveTabIcon };
}

export type { IBaseContext, IBaseConfig, ComponentRenderer };

export { useSplitPanels };
