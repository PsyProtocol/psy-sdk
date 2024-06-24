import { ICodeEditorProps, IControlledCodeEditorProps, genLazyLoadMonacoCodeEditor, genLazyLoadMonacoControlledCodeEditor } from "@qstudio/monaco-textmate-lazy";
import { DEFAULT_MONACO_SETUP_CONFIG } from "./config";
import LazyLoader from "../LazyLoader";

const MonacoCodeEditor: React.FC<ICodeEditorProps> = genLazyLoadMonacoCodeEditor(DEFAULT_MONACO_SETUP_CONFIG, LazyLoader);
const MonacoControlledCodeEditor: React.FC<IControlledCodeEditorProps> = genLazyLoadMonacoControlledCodeEditor(DEFAULT_MONACO_SETUP_CONFIG, LazyLoader);

export {
  MonacoCodeEditor,
  MonacoControlledCodeEditor,
}