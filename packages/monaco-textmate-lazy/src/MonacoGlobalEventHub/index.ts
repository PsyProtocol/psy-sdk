import { EventHub } from "@qstudio/utils";
import { MonacoGlobalEvent, MonacoGlobalEventType } from "./types";

const monacoGlobalEventHub = new EventHub<MonacoGlobalEventType, MonacoGlobalEvent>();

function notifyMonacoResize(){
  monacoGlobalEventHub.notify({type: MonacoGlobalEventType.PatchMonaco, patch: (monaco)=>{
    monaco.editor.getEditors().forEach(editor=>{
      editor.layout();
    });
  }});
}

export {
  monacoGlobalEventHub,
  notifyMonacoResize,
}