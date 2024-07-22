
import {FileExplorerConfig} from "@qstudio/file-explorer";
import { AsyncFileStore, ISyncFileStore, MemoryFileStore, SyncCombinedFileStore } from "@qstudio/storage";
import { BlockVizEventType, EditorLogEventType, EditorLogLevel, EditorLogMessageType, EditorUIEventType, IBlockVizEvent, IDEMenuId, IEditorLogEvent, IEditorLogEventClear, IEditorLogMessageEvent, IEditorUIEvent, ProjectFilesEvent, ProjectFilesEventType, SplitPanelsEvent, SplitPanelsEventType } from "@qstudio/eventhubs";
import { EventHub } from "@qstudio/utils";
import localforage from "localforage";
import { FileIconMap } from "../dockComponents";
import { SlDoc, SlFolder } from "react-icons/sl";
import { GlobalProjectManager } from "./projectManager";
import { MonacoGlobalEventType, monacoGlobalEventHub } from "@qstudio/monaco-textmate-lazy";
import { CityRPCProvider } from "@qstudio/city-sdk";
import { DogeLinkElectrsComboRPC, IDogeLinkElectrsRPC } from "doge-sdk";
import { BlockVizDataStore } from "./blockviz/BlockVizDataStore";


class IDEContext {

  fileExplorerConfig: FileExplorerConfig;
  fileStorage: ISyncFileStore;
  splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>;
  blockVizEventHub: EventHub<BlockVizEventType, IBlockVizEvent>;
  logMessages: IEditorLogMessageEvent[] = [];
  activeFile: string = "";
  projectManager: GlobalProjectManager;
  dogeRPC: DogeLinkElectrsComboRPC = new DogeLinkElectrsComboRPC("http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest", "http://localhost:1337/api");
  rpc: CityRPCProvider = new CityRPCProvider("http://localhost:3000");
  blockVizDataStore: BlockVizDataStore;
  constructor(fileExplorerConfig: FileExplorerConfig, fileStorage: ISyncFileStore, splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>, logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>, blockVizEventHub: EventHub<BlockVizEventType, IBlockVizEvent>, projectManager: GlobalProjectManager) {
    this.fileExplorerConfig = fileExplorerConfig;
    this.fileStorage = fileStorage;
    this.splitPanelsEventHub = splitPanelsEventHub;
    this.fileEventHub = fileExplorerConfig.fileEventHub;
    this.logEventHub = logEventHub;
    this.projectManager = projectManager;
    this.blockVizEventHub = blockVizEventHub;
    this.blockVizDataStore = new BlockVizDataStore(this.dogeRPC, this.rpc, blockVizEventHub);

    this.onLogMessage = this.onLogMessage.bind(this);
    this.onLogClear = this.onLogClear.bind(this);
    this.onDocumentKeyDown = this.onDocumentKeyDown.bind(this);


    this.setupEventListeners();
  }

  onLogMessage(e: IEditorLogMessageEvent){
    this.logMessages.push(e);
  }
  onLogClear(e: IEditorLogEventClear){
    this.logMessages = [];
  }

  onDocumentKeyDown(e: KeyboardEvent){
    if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      e.stopPropagation();
      this.openCommandBar();
      return false;
    }else if (e.key === 'p' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      e.stopPropagation();
      this.openCommandBar(IDEMenuId.Files);
      return false;
    }
  }

  openCommandBar(menu?: IDEMenuId, originId?: string){
    this.projectManager.uiEventHub.notify({type: EditorUIEventType.CommandBar, menuType: menu || IDEMenuId.Standard, originId: originId});
  }

  private setupEventListeners(){
    // setup log
    this.logEventHub.on(EditorLogEventType.Message, this.onLogMessage);
    this.logEventHub.on(EditorLogEventType.Clear, this.onLogClear);

    // setup keyboard monitor
    document.addEventListener("keydown", this.onDocumentKeyDown);
  }

  private cleanUpEventListeners(){
    this.logEventHub.remove(EditorLogEventType.Message, this.onLogMessage);
    this.logEventHub.remove(EditorLogEventType.Clear, this.onLogClear);
    document.removeEventListener("keydown", this.onDocumentKeyDown);
  }


  static async newContext(projectManager: GlobalProjectManager): Promise<IDEContext> {
    const logEventHub = new EventHub<EditorLogEventType, IEditorLogEvent>();
    const fileEventHub = new EventHub<ProjectFilesEventType, ProjectFilesEvent>();
    const blockVizEventHub = new EventHub<BlockVizEventType, IBlockVizEvent>();
    const keyPrefix = projectManager.activeProject?.id || "NO_PROJECT";
    const localForageInstance = localforage.createInstance({name: "IDE_DEMO"});
    const asyncFileStorage = new AsyncFileStore(keyPrefix, localForageInstance);
    const syncFileStorage = new MemoryFileStore(keyPrefix, fileEventHub);
    const combinedFileStorage = new SyncCombinedFileStore(asyncFileStorage, syncFileStorage);
    await combinedFileStorage.refreshFromAsyncStore();

    const fileExplorerConfig = new FileExplorerConfig({
      fileIcons: FileIconMap,
      defaultFileIcon: {icon: SlDoc, iconColor: "#f0f0f0"},
      defaultFolderIcon: {icon: SlFolder, iconColor: "#f0f0f0"},
      fileEventHub: fileEventHub,
      store: combinedFileStorage,
    });

    const splitPanelsEventHub = new EventHub<SplitPanelsEventType, SplitPanelsEvent>();

    return new IDEContext(fileExplorerConfig, combinedFileStorage, splitPanelsEventHub, logEventHub, blockVizEventHub, projectManager);
  }
  print(message: string, logLevel: EditorLogLevel = EditorLogLevel.Info) {
    this.logEventHub.notify(EditorLogEventType.Message, {type: EditorLogEventType.Message, messageType: EditorLogMessageType.PlainText, level: logLevel, message});
  }
  println(message: string, logLevel: EditorLogLevel = EditorLogLevel.Info) {
    return this.print(message+"\n", logLevel);
  }
  openFile(filePath: string){
    this.splitPanelsEventHub.notify({type: SplitPanelsEventType.OpenFileTab, filePath: filePath});
  }
  setActiveFile(filePath?: string){
    this.activeFile = filePath || "";

  }
}

export {
  IDEContext,
}