import * as monaco from "monaco-editor";
import { monacoGlobalEventHub } from "../MonacoGlobalEventHub";
import { IPatchMonacoEvent, MonacoGlobalEventType } from "../MonacoGlobalEventHub/types";
import { IMonacoGlobalSetupConfig } from "../types";
import { getCurrentRawTheme, setMonacoTheme } from "./theme";
import { provider } from "./loadMonacoTextmate";
let hasSetupMonacoEventHandlers = false;
function setupMonacoEventHandlers(config: IMonacoGlobalSetupConfig) {
  if(!hasSetupMonacoEventHandlers){
    hasSetupMonacoEventHandlers = true;
    monacoGlobalEventHub.on(MonacoGlobalEventType.PatchMonaco, (event: IPatchMonacoEvent)=>{
      event.patch(monaco);
    });
    monacoGlobalEventHub.on(MonacoGlobalEventType.ResizeEditors, ()=>{
      monaco.editor.getEditors().forEach(editor=>{
        editor.layout();
      });
    });
    monacoGlobalEventHub.on(MonacoGlobalEventType.SwitchTheme, (event)=>{
      if(event.url){
        setMonacoTheme(event.theme, event.url).then(()=>provider?.switchTheme(getCurrentRawTheme()!)).catch(console.error);
      }else{
        const urlInConfig = config.themes.filter(x=>x.name === event.theme)[0]?.url;
        if(urlInConfig){
          setMonacoTheme(event.theme, urlInConfig).then(()=>provider?.switchTheme(getCurrentRawTheme()!)).catch(console.error);
        }
      }
    });
  }
}


export {setupMonacoEventHandlers};
