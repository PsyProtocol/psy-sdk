import React, {useRef, useEffect} from 'react';
import * as monaco from 'monaco-editor';
import { IControlledCodeEditorProps } from '../../types';

export const ControlledCodeEditor: React.FC<IControlledCodeEditorProps> = ({ className, value, onChange, options, keyBindings }) => {

  const nodeRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>();
  const language = options?.language || "";
  useEffect(() => {
    if (!nodeRef.current || editorRef.current) return;
    const x = monaco.editor.create(nodeRef.current, {
      ...(options||{}),
      language,
    });

    if(keyBindings){
      keyBindings.forEach(kb=>{
        x.addCommand(kb.key, kb.action);
      })
    }

    editorRef.current = x;
    editorRef.current.onDidChangeModelContent((e) => {
      const v = editorRef.current?.getModel()?.getValue();
      if (typeof v === 'string' && v !== value) {
        onChange(v)
      }
    })
  }, [nodeRef.current]);
  useEffect(() => {
    const c = editorRef.current;
    if (c) {
      const m = c.getModel();
      if (m) {
        monaco.editor.setModelLanguage(m, language);
      }
    }
  }, [language]);
  useEffect(() => {
    const c = editorRef.current;
    if (c) {
      const m = c.getModel();
      if (m) {
        if (m.getValue() !== value) {
          m.setValue(value);
        }
      }
    }
  }, [value]);


  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        maxWidth: "100%",
        maxHeight: "100%",
        background: "#292929"

      }}
      data-tabster='{"uncontrolled": {}}'
      className={className}
      ref={nodeRef}
    />
  );
};