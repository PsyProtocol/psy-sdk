import {useEffect, useMemo, useState} from 'react';
import { SplitPanelsManaged } from '@qstudio/split-panels';
import styles from './ide.module.scss';
import { DEFAULT_DOCK_MODEL } from './dockModel';
import { IDEContext } from '../../utils/ideContext';
import { CoreEditorConfig } from '../../dockComponents';
import { loadProjectIfEmpty } from '../../utils/loadProjectIfEmpty';
import { sampleProject } from '../../utils/sampleProject';
import { useActiveFile } from '../../hooks/useActiveFile';
import { monacoGlobalEventHub, notifyMonacoResize } from '@qstudio/monaco-textmate-lazy';
import { debounce } from '@qstudio/utils';
import { GlobalProjectManager } from '../../utils/projectManager';
import { EditorUIEventType, IEditorUIOpenProjectEvent } from 'packages/eventhubs/src/EditorUI';
import { CommandBarModal } from '../../components/CommandBarModal';
import { IDEMenuGenerators } from '../../commands/registry';


const debouncedResize = debounce(notifyMonacoResize, 100, {immediate: true});
function randHex(len: number){
  let s = "";
  for(let i=0;i<len;i++){
    s += Math.floor(Math.random()*16).toString(16);
  }
  return s;

}
const IDEPage: React.FC = () => {
  const [ctx, setCtx] = useState<IDEContext>();
  const [projectManager, setProjectManager] = useState<GlobalProjectManager>();
  const setActiveFile = useActiveFile(s=>s.setActiveFile);

  useEffect(()=>{
    if(!projectManager){
      GlobalProjectManager.init().then((pm)=>{
        setProjectManager(pm);
        setCtx(pm.activeIDEContext);

      }).catch((e)=>{
        console.error("ERROR: ",e);

      });
      return () => {

      };
    }else{
      const onOpenProject = (e: IEditorUIOpenProjectEvent) => {
        setCtx(projectManager.activeIDEContext);
      }
      projectManager.uiEventHub.on(EditorUIEventType.OpenProject, onOpenProject);
      return () => {
        projectManager.uiEventHub.remove(EditorUIEventType.OpenProject, onOpenProject);
      };
    }
  },[projectManager]);
  if(!ctx) return (<div>Loading...</div>);
  
  return(
    <div className={styles.idePage}>
      <div className={styles.idePageTopBar}>
        Top <button onClick={()=>{
          if(ctx){
            const len = Math.pow(Math.random(),1.5)*5000;
            ctx.println(`length: ${len} ${randHex(len)}`);
          }
        }}>Log</button>
      </div>
      <div className={styles.idePageContent}>
        <SplitPanelsManaged modelJson={DEFAULT_DOCK_MODEL} config={CoreEditorConfig as any} editorContext={ctx} onActiveFileChanged={(f)=>{
          ctx.setActiveFile(f);
          setActiveFile(f);
        }} notifyResize={()=>{
          debouncedResize();
        }} /> 
      </div>
      <CommandBarModal ctx={ctx} menuGenerator={IDEMenuGenerators} />
    </div>
  )
};

export default IDEPage;