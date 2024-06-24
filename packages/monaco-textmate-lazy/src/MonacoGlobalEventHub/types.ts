import type * as monaco from 'monaco-editor';

type MonacoGlobalType = typeof monaco;
type MonacoEditorGlobalType = typeof monaco.editor;
enum MonacoGlobalEventType {
  PatchMonaco = 'patch-monaco',
  LoadingStateChanged = 'loading-state-changed',
  ResizeEditors = 'resize-editors',
  SwitchTheme = 'switch-theme',
}

enum MonacoLoadingState {
  NotLoaded = 0,
  Loading = 1,
  Loaded = 2,
  Error = 3,
}


interface IMonacoGlobalEventBase {
  type: MonacoGlobalEventType;
}

interface IPatchMonacoEvent extends IMonacoGlobalEventBase {
  type: MonacoGlobalEventType.PatchMonaco;
  patch: (monaco: MonacoGlobalType)=>void;
}

interface IMonacoLoadingStateChangedEvent extends IMonacoGlobalEventBase {
  type: MonacoGlobalEventType.LoadingStateChanged;
  state: MonacoLoadingState;
  monaco?: MonacoGlobalType;
}

interface IResizeMonacoEditorsEvent extends IMonacoGlobalEventBase {
  type: MonacoGlobalEventType.ResizeEditors;
}

interface ISwitchThemeMonacoEvent extends IMonacoGlobalEventBase {
  type: MonacoGlobalEventType.SwitchTheme;
  theme: string;
  url?: string;
}

type MonacoGlobalEvent = IPatchMonacoEvent | IMonacoLoadingStateChangedEvent | IResizeMonacoEditorsEvent | ISwitchThemeMonacoEvent;

export {
  MonacoGlobalEventType,
  MonacoLoadingState,
}

export type {
  MonacoGlobalEvent,
  IPatchMonacoEvent,
  IMonacoLoadingStateChangedEvent,
  IResizeMonacoEditorsEvent,
  MonacoGlobalType,
  MonacoEditorGlobalType,
}