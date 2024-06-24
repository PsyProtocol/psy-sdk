import { useEffect, useMemo } from 'react';
import { EventHub, uuidv4 } from '@qstudio/utils';
import { useRenamableFile } from '../../hooks/useRenamableFile';
import styles from './CodeEditor.module.scss';
import { EditorUIEventType, IDEMenuId, ProjectFilesEvent, ProjectFilesEventType } from '@qstudio/eventhubs';
import { IPatchMonacoEvent, MonacoGlobalEventType, MonacoKeyCode, MonacoKeyMod, monacoGlobalEventHub } from '@qstudio/monaco-textmate-lazy';
import { MonacoCodeEditor } from '../../components/CodeEditor';
import { ISyncFileStore } from '@qstudio/storage';
import { IDEContext } from '../../utils/ideContext';
import { getFileExtForFileName, getLanguageForFilePath } from '../../utils/fileIcons';

interface ICodeEditorDockComponentProps {
  filePath: string;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  fileStore: ISyncFileStore;
  ctx: IDEContext;
}

const CodeEditorDockComponent: React.FC<ICodeEditorDockComponentProps> = ({ filePath, fileEventHub, fileStore, ctx }) => {

  const originId = useMemo<string>(() => uuidv4(), []);
  const realFilePath = useRenamableFile(filePath, fileEventHub);
  const realFilePathWithFrontSlash = "/"+realFilePath;

  const keyBindings = useMemo(() => {
    return [
      {
        key: MonacoKeyMod.CtrlCmd | MonacoKeyCode.KeyK, 
        action: () => ctx.openCommandBar(IDEMenuId.Standard, originId),
      },
      {
        key: MonacoKeyMod.CtrlCmd | MonacoKeyCode.KeyP, 
        action: () => ctx.openCommandBar(IDEMenuId.Files, originId),
      },
    ];
  }, [originId]);

  useEffect(() => {
    const dispose = ctx.projectManager.uiEventHub.onceFilter(EditorUIEventType.CloseCommandBar, (e) => e.originId === originId, (e) => {
      monacoGlobalEventHub.notify({
        type: MonacoGlobalEventType.PatchMonaco, patch: (monaco) => {
          const editors = monaco.editor.getEditors();
          for (const editor of editors) {
            const editorModelPath = editor.getModel()?.uri.path;
            if (editorModelPath === realFilePath || editorModelPath === realFilePathWithFrontSlash) {
              editor.focus();
            }
          }
        }
      });
    });
    return () => {
      dispose();
    }
  }, [originId, realFilePath]);

  return (
    <div className={styles.codeEditorDockPage}>
      <MonacoCodeEditor
        filePath={realFilePath}
        options={{ language: getLanguageForFilePath(realFilePath) }}
        fileStore={fileStore}
        keyBindings={keyBindings}
      />
    </div>
  )
};

export default CodeEditorDockComponent;