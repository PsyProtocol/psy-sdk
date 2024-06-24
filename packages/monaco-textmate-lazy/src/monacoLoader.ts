import { monacoGlobalEventHub } from "./MonacoGlobalEventHub";
import { MonacoGlobalEventType, MonacoGlobalType } from "./MonacoGlobalEventHub/types";
import { ICodeEditorProps, IControlledCodeEditorProps, IMonacoGlobalSetupConfig } from "./types";

enum MonacoLoadingState {
  NotLoaded = 0,
  Loading = 1,
  Loaded = 2,
  Error = 3,
}

let currentLoadingState: MonacoLoadingState = MonacoLoadingState.NotLoaded;


let LoadedControlledCodeEditor: React.FC<IControlledCodeEditorProps> = null as any;
let LoadedCodeEditor: React.FC<ICodeEditorProps> = null as any;


function setMonacoLoadingState(value: MonacoLoadingState, monaco?: MonacoGlobalType) {
  currentLoadingState = value;
  monacoGlobalEventHub.notify({type: MonacoGlobalEventType.LoadingStateChanged, state: value, monaco});
}

function getMonacoLoadingState() {
  return currentLoadingState;
}

async function loadMonaco(config: IMonacoGlobalSetupConfig) {
  if (currentLoadingState === MonacoLoadingState.Loaded) {
    return;
  }
  if (currentLoadingState === MonacoLoadingState.Loading) {
    return;
  }
  setMonacoLoadingState(MonacoLoadingState.Loading);
  try {
    const entrypoint = await import("./entrypoint");
    const monaco = await entrypoint.setupMonaco(config);
    LoadedCodeEditor = entrypoint.CodeEditor;
    LoadedControlledCodeEditor = entrypoint.ControlledCodeEditor;
    setMonacoLoadingState(MonacoLoadingState.Loaded, monaco);
    monacoGlobalEventHub.notify({type: MonacoGlobalEventType.ResizeEditors });
    setTimeout(()=>{
      monacoGlobalEventHub.notify({type: MonacoGlobalEventType.ResizeEditors });
    },300);
  } catch (e) {
    setMonacoLoadingState(MonacoLoadingState.Error);
    console.error(e);
  }
}

function getLoadedControlledCodeEditor(): React.FC<IControlledCodeEditorProps> {
  return LoadedControlledCodeEditor;
}

function getLoadedCodeEditor(): React.FC<ICodeEditorProps> {
  return LoadedCodeEditor;
}


export {
  getMonacoLoadingState,
  loadMonaco,
  getLoadedControlledCodeEditor,
  getLoadedCodeEditor,
}