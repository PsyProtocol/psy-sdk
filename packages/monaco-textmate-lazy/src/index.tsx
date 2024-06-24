export { genLazyLoadMonacoCodeEditor, genLazyLoadMonacoComponent, genLazyLoadMonacoControlledCodeEditor } from './lazyLoadComponent';

export {MonacoGlobalEventType} from './MonacoGlobalEventHub/types';

export type {
  MonacoGlobalEvent,
  IPatchMonacoEvent,
  IMonacoLoadingStateChangedEvent,
  IResizeMonacoEditorsEvent,
  MonacoGlobalType,
  MonacoEditorGlobalType,
} from './MonacoGlobalEventHub/types';

export {monacoGlobalEventHub, notifyMonacoResize} from './MonacoGlobalEventHub';

export {IndentAction} from './types';

export type {
  ScopeNameInfo,
  RemoteScopeNameInfo,
  IRemoteThemeInfo,
  IMonacoGlobalSetupConfig,
  ICodeEditorProps,
  IControlledCodeEditorProps,
} from './types';

export {
  MonacoKeyCode,
  MonacoKeyMod,
} from './keyBindings';