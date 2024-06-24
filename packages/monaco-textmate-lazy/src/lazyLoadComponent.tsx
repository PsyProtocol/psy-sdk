import React, { useEffect, useState } from "react";
import { getLoadedCodeEditor, getLoadedControlledCodeEditor, getMonacoLoadingState, loadMonaco } from "./monacoLoader";
import { IMonacoLoadingStateChangedEvent, MonacoGlobalEventType, MonacoLoadingState } from "./MonacoGlobalEventHub/types";
import { monacoGlobalEventHub } from "./MonacoGlobalEventHub";
import { ICodeEditorProps, IControlledCodeEditorProps, IMonacoGlobalSetupConfig } from "./types";
function genLazyLoadMonacoComponent<P>(resolver: () => React.FC<P>, config: IMonacoGlobalSetupConfig, LoadingComponent: React.FC) {
  const BaseComponent: any = resolver();
  if (BaseComponent) {
    return BaseComponent;
  } else {
    const LazyComponent: React.FC<P> = (props) => {
      let [loadingState, setLoadingState] = useState(() => getMonacoLoadingState());
      useEffect(() => {
        const loadingStateEventListener = (e: IMonacoLoadingStateChangedEvent) => {
          setLoadingState(e.state);
        };
        monacoGlobalEventHub.on(MonacoGlobalEventType.LoadingStateChanged, loadingStateEventListener);
        loadMonaco(config).catch(console.error);

        return () => {
          monacoGlobalEventHub.removeEventListener(MonacoGlobalEventType.LoadingStateChanged, loadingStateEventListener);
        };
      }, []);
      if (loadingState === MonacoLoadingState.Loaded) {
        const NewBaseComponent: any = resolver();
        return <NewBaseComponent {...props} />;
      } else {
        return (
          <LoadingComponent />
        )
      }
    };
    return LazyComponent;
  }
}

function genLazyLoadMonacoCodeEditor(config: IMonacoGlobalSetupConfig, LoadingComponent: React.FC): React.FC<ICodeEditorProps>{
  return genLazyLoadMonacoComponent(getLoadedCodeEditor, config, LoadingComponent);
}

function genLazyLoadMonacoControlledCodeEditor(config: IMonacoGlobalSetupConfig, LoadingComponent: React.FC): React.FC<IControlledCodeEditorProps>{
  return genLazyLoadMonacoComponent(getLoadedControlledCodeEditor, config, LoadingComponent);
}

export {
  genLazyLoadMonacoComponent,
  genLazyLoadMonacoCodeEditor,
  genLazyLoadMonacoControlledCodeEditor,
}