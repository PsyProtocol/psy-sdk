
import {FileExplorerConfig} from "@qstudio/file-explorer";
import { AsyncFileStore, ISyncFileStore, MemoryFileStore, SyncCombinedFileStore } from "@qstudio/storage";
import { EditorLogEventType, EditorLogLevel, EditorLogMessageType, EditorUIEventType, IDEMenuId, IEditorLogEvent, IEditorLogEventClear, IEditorLogMessageEvent, IEditorUIEvent, ProjectFilesEvent, ProjectFilesEventType, SplitPanelsEvent, SplitPanelsEventType } from "@qstudio/eventhubs";
import { EventHub } from "@qstudio/utils";
import localforage from "localforage";
import { FileIconMap } from "../dockComponents";
import { SlDoc, SlFolder } from "react-icons/sl";
import { GlobalProjectManager } from "./projectManager";
import { MonacoGlobalEventType, monacoGlobalEventHub } from "@qstudio/monaco-textmate-lazy";


class IDEContext {

  fileExplorerConfig: FileExplorerConfig;
  fileStorage: ISyncFileStore;
  splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>;
  logMessages: IEditorLogMessageEvent[] = [];
  activeFile: string = "";
  projectManager: GlobalProjectManager;
  constructor(fileExplorerConfig: FileExplorerConfig, fileStorage: ISyncFileStore, splitPanelsEventHub: EventHub<SplitPanelsEventType, SplitPanelsEvent>, logEventHub: EventHub<EditorLogEventType, IEditorLogEvent>, projectManager: GlobalProjectManager) {
    this.fileExplorerConfig = fileExplorerConfig;
    this.fileStorage = fileStorage;
    this.splitPanelsEventHub = splitPanelsEventHub;
    this.fileEventHub = fileExplorerConfig.fileEventHub;
    this.logEventHub = logEventHub;
    this.projectManager = projectManager;

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

    return new IDEContext(fileExplorerConfig, combinedFileStorage, splitPanelsEventHub, logEventHub, projectManager);
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