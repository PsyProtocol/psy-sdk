import React, { useRef, useState, useEffect } from 'react';
import * as monaco from 'monaco-editor';
import { ICodeEditorProps } from '../../types';
import { notifyMonacoResize } from '../../MonacoGlobalEventHub';

export const CodeEditor: React.FC<ICodeEditorProps> = ({ filePath, fileStore, options, keyBindings, className }: ICodeEditorProps) => {

  const nodeRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const fileName = filePath.split("/").pop() || "";

  useEffect(() => {
    if (!nodeRef.current) return;
    if (editorRef.current) {
      editorRef.current.dispose();
    }
    const uri = monaco.Uri.parse('file:///' + filePath);
    const model = monaco.editor.getModel(uri) || (
      filePath ? monaco.editor.createModel(fileStore.getFileContent(filePath) || "", options?.language, uri) : undefined
    );
    const x = monaco.editor.create(nodeRef.current, {
      model,
      ...(options || {}),
    },
    );
    editorRef.current = x;

    if (keyBindings) {
      keyBindings.forEach(kb => {
        x.addCommand(kb.key, kb.action);
      })
    }

    /*if (filePath && x) {
      const v = fileStore.getFileContent(filePath) || "";
      if (typeof v === 'string') {
        const m = x.getModel();
        if (m) {
          m.setValue(v);
        }
      }
    }*/
    notifyMonacoResize();

    let r = editorRef.current.onDidChangeModelContent((e) => {
      const v = editorRef.current?.getModel()?.getValue();
      if (typeof v === 'string') {
        fileStore.setFile(filePath, v);
      }

    })
    return () => {
      r.dispose();
      x.dispose();
      if (editorRef.current) {
        editorRef.current.dispose();
      }
      editorRef.current = undefined;
    };
  }, [nodeRef.current, filePath, keyBindings]);

  return (
    <div
      className={className}
      style={{
        width: "100%",
        height: "100%",
        maxWidth: "100%",
        maxHeight: "100%",
        background: "#222"

      }}
      ref={nodeRef}
    />
  );
}
