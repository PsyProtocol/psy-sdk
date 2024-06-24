import { IMonacoGlobalSetupConfig } from "../types";
import setupVSCodeTextmate from "./loadMonacoTextmate";
import { setupMonacoEventHandlers } from "./monacoEventHandlers";
import { getCurrentRawTheme, setMonacoTheme } from "./theme";
import * as monaco from "monaco-editor";
let setup = false;

let isLoading = false;

export function isWorkspaceSetup() {
  return setup;
}

async function setupMonaco(config: IMonacoGlobalSetupConfig){
  if(setup || isLoading){
    return;
  }
  isLoading = true;

  if(config.defaultTheme){
    const url = config.themes.filter(x=>x.name === config.defaultTheme)[0]?.url;
    if(url){
      await setMonacoTheme(config.defaultTheme, url);
    }
  }
  const provider = await setupVSCodeTextmate(config);
 
  provider.injectCSS();
  if(config.finishMonacoSetup){
    await config.finishMonacoSetup(monaco);
  }
  setupMonacoEventHandlers(config);

  if(config.defaultTheme){
    const url = config.themes.filter(x=>x.name === config.defaultTheme)[0]?.url;
    if(url){
      await setMonacoTheme(config.defaultTheme, url);
      provider.switchTheme(getCurrentRawTheme()!);
    }
  }

  setup = true;
  isLoading = false;
  return monaco;
}
export {
  setupMonaco,
}