import type { MonacoGlobalType, MonacoEditorGlobalType } from "./MonacoGlobalEventHub/types";
import type { languages, editor } from "monaco-editor";

enum IndentAction {
  /**
   * Insert new line and copy the previous line's indentation.
   */
  None = 0,
  /**
   * Insert new line and indent once (relative to the previous line's indentation).
   */
  Indent = 1,
  /**
   * Insert two new lines:
   *  - the first one indented which will hold the cursor
   *  - the second one at the same indentation level
   */
  IndentOutdent = 2,
  /**
   * Insert new line and outdent once (relative to the previous line's indentation).
   */
  Outdent = 3
}
interface ScopeNameInfo {
  /**
   * If set, this is the id of an ILanguageExtensionPoint. This establishes the
   * mapping from a MonacoLanguage to a TextMate grammar.
   */
  language?: string;

  /**
   * Scopes that are injected *into* this scope. For example, the
   * `text.html.markdown` scope likely has a number of injections to support
   * fenced code blocks.
   */
  injections?: string[];
}

interface RemoteScopeNameInfo extends ScopeNameInfo {
  url: string;
}

interface IRemoteThemeInfo {
  name: string;
  url: string;
}
interface IAdditionalLanguage {
  id: string;
  filenamePatterns: string[];
}
interface IMonacoGlobalSetupConfig {
  languages: languages.ILanguageExtensionPoint[];
  grammars: Record<string, RemoteScopeNameInfo>;
  languageConfiguration: Record<string, languages.LanguageConfiguration>;
  themes: IRemoteThemeInfo[];
  defaultTheme?: string;
  additionalLanguages?: IAdditionalLanguage[];
  onigurumaWasmUrl: string;
  finishMonacoSetup?: (monaco: MonacoGlobalType)=>Promise<any>;
}

interface ISyncFileStore {
  getFilePaths(): string[];
  ensureFile(path: string, defaultContent?: string): void;
  addFile(path: string, content?: string): void;
  setFile(path: string, content: string): void;
  addFiles(files: {path: string, content?: string}[]): void;
  renameFile(oldPath: string, newPath: string): void;
  renameFiles(targets: {oldPath: string, newPath: string}[]): void;
  deleteFile(path: string): void;
  deleteFiles(path: string[]): void;
  createFolder(folder: string): void;
  deleteFolder(folder: string): void;
  renameFolder(oldPath: string, newPath: string): void;
  getFileContent(path: string): string;
  getAllFiles(): {path: string, content: string}[];
  withEventSource(eventSource: string): ISyncFileStore;
}

interface IControlledCodeEditorProps {
  className?: string;
  value: string;
  onChange: (v: string) => any;
  options?: editor.IStandaloneEditorConstructionOptions;
  keyBindings?: ICodeEditorKeyBinding[];


}

interface ICodeEditorKeyBinding {
  key: number;
  action: (editor: MonacoEditorGlobalType, monaco: MonacoGlobalType) => any; 
}

interface ICodeEditorProps {
  className?: string;
  filePath: string;
  fileStore: ISyncFileStore;
  options?: editor.IStandaloneEditorConstructionOptions;
  keyBindings?: ICodeEditorKeyBinding[];
}

export {IndentAction};
export type {
  ScopeNameInfo,
  RemoteScopeNameInfo,
  IRemoteThemeInfo,
  IMonacoGlobalSetupConfig,
  IControlledCodeEditorProps,
  ICodeEditorKeyBinding,
  ICodeEditorProps,
};