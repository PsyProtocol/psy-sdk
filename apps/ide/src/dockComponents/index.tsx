import { SlDoc } from "react-icons/sl";
import FileExplorerDockComponent from "./FileExplorer";
import LogDockComponent from "./Log";
import WelcomeDockComponent from "./Welcome";
import { IDEDockComponents } from "./types";
import {Layout, Model} from "@qstudio/flex-layout";
import { SiHtml5, SiJavascript, SiCss3,SiTypescript, SiBitcoin, SiMarkdown } from "react-icons/si";
import { VscFiles, VscTerminal } from "react-icons/vsc";

import { GiTheaterCurtains, GiWallet } from "react-icons/gi";

import { GoFile } from "react-icons/go";
import { BsCardText } from "react-icons/bs";
import CodeEditorDockComponent from "./CodeEditor";
import {FileExplorerConfig} from "@qstudio/file-explorer";
import { ISyncFileStore } from "@qstudio/storage";
import {ComponentRenderer, IBaseConfig} from "@qstudio/split-panels";
import { EditorLogEventType, IEditorLogEvent, ProjectFilesEvent, ProjectFilesEventType, SplitPanelsEvent, SplitPanelsEventType } from "@qstudio/eventhubs";
import { EventHub } from "@qstudio/utils";
import { IDEContext } from "../utils/ideContext";
import StageDockComponent from "./Stage";
import BlockPlannerDockComponent from "./BlockPlanner";
import WalletDockComponent from "./Wallet";
import CityReplDockComponent from "./CityRepl";
/*
interface IEditorContext {
  fileExplorerConfig: FileExplorerConfig;
  fileStorage: ISyncFileStore;
  splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>;
}
*/
type IEditorContext = IDEContext;
interface IEditorConfig extends IBaseConfig<IDEDockComponents, IEditorContext> {

}

const FileIconMap: any = {
  "js": {icon: SiJavascript, iconColor: "#efd81e"},
  "ts": {icon: SiTypescript, iconColor: "#007acc"},
  "jsx": {icon: SiJavascript, iconColor: "#efd81e"},
  "tsx": {icon: SiTypescript, iconColor: "#007acc"},
  "basm":{
    icon: SiBitcoin,
    iconColor: "#f7931a",
  },
  "html": {icon: SiHtml5, iconColor: "#f06529"},
  "css": {icon: SiCss3, iconColor: "#2965f1"},
  "md": {icon: SiMarkdown, iconColor: "#f0f0f0"},
}
const DockComponentMap : Record<IDEDockComponents, ComponentRenderer<IEditorContext>> = {
  [IDEDockComponents.FileExplorer]: ({ }, { layout, model, editorContext }) => (<FileExplorerDockComponent config={editorContext.fileExplorerConfig} onFileSelected={(filePath) => {
    editorContext.splitPanelsEventHub.notify({ type: SplitPanelsEventType.OpenFileTab, filePath: filePath });

  } } />),
  [IDEDockComponents.Log]: (_, { editorContext }) => <LogDockComponent ideContext={editorContext} />,
  [IDEDockComponents.Welcome]: () => <WelcomeDockComponent />,
  [IDEDockComponents.CodeEditor]: (args: { filePath: string; }, { editorContext }) => (<CodeEditorDockComponent fileStore={editorContext.fileStorage} fileEventHub={editorContext.fileEventHub} filePath={args.filePath} ctx={editorContext} />),
  [IDEDockComponents.Stage]: (args: { filePath: string; }, { editorContext }) => (<StageDockComponent fileStore={editorContext.fileStorage} fileEventHub={editorContext.fileEventHub} filePath={args.filePath} ctx={editorContext} />),
  [IDEDockComponents.BlockPlanner]: (_, { editorContext }) => (<BlockPlannerDockComponent ctx={editorContext} />),
  [IDEDockComponents.Wallet]: (_, { editorContext }) => (<WalletDockComponent ctx={editorContext} />),
  [IDEDockComponents.CityRepl]: (_, { editorContext }) => (<CityReplDockComponent ctx={editorContext} />),


};

const DockIconMap : Record<IDEDockComponents, JSX.Element> = {
  [IDEDockComponents.Welcome]: <span />,
  [IDEDockComponents.FileExplorer]: <VscFiles width={"1em"} height={"1em"} />,
  [IDEDockComponents.Log]: <BsCardText width={16} height={16} />,
  [IDEDockComponents.CodeEditor]: <BsCardText width={16} height={16} />,
  [IDEDockComponents.Stage]: <GiTheaterCurtains width={16} height={16} />,
  [IDEDockComponents.BlockPlanner]: <GiTheaterCurtains width={16} height={16} />,
  [IDEDockComponents.Wallet]: <GiWallet width={16} height={16} />,
  [IDEDockComponents.CityRepl]: <VscTerminal width={"1em"} height={"1em"} />,
};


const CoreEditorConfig: IEditorConfig = {
  editorComponentType: IDEDockComponents.CodeEditor,
  tabComponentIconMap: {
    [IDEDockComponents.CodeEditor]: { icon: SlDoc, iconColor: "#f0f0f0" },
    [IDEDockComponents.FileExplorer]: { icon: VscFiles },
    [IDEDockComponents.Log]: { icon: BsCardText },
    [IDEDockComponents.Welcome]: { icon: BsCardText },
    [IDEDockComponents.Stage]: {icon: GiTheaterCurtains},
    [IDEDockComponents.BlockPlanner]: {icon: GiTheaterCurtains},
    [IDEDockComponents.Wallet]: {icon: GiWallet},
    [IDEDockComponents.CityRepl]: {icon: VscTerminal},
  },
  tabComponentTitleMap: {
    [IDEDockComponents.CodeEditor]: "Code Editor",
    [IDEDockComponents.FileExplorer]: "File Explorer",
    [IDEDockComponents.Log]: "Log",
    [IDEDockComponents.Welcome]: "Welcome",
    [IDEDockComponents.Stage]: "Stage",
    [IDEDockComponents.BlockPlanner]: "Block Planner",
    [IDEDockComponents.Wallet]: "Wallet",
    [IDEDockComponents.CityRepl]: "City REPL",
  },
  fileExtensionIconMap: FileIconMap,
  panelComponentMap: DockComponentMap,
}



function getComponentForDockComponent(dc: IDEDockComponents | string): ComponentRenderer<IEditorContext> {
  if(Object.hasOwnProperty.call(DockComponentMap, dc as any)){
    return DockComponentMap[dc as IDEDockComponents] || DockComponentMap[IDEDockComponents.Welcome];
  }else{
    return DockComponentMap[IDEDockComponents.Welcome];
  }
}


export {
  getComponentForDockComponent,
  IDEDockComponents,
  DockIconMap,
  DockComponentMap,
  CoreEditorConfig,
  FileIconMap,
}