import { IQVizStyleResolver, QVizStyleResolver } from "@qstudio/core";
import { qwCityProofStyleDef } from "./QWCityProof";

function setupWidgetStyles(registry: QVizStyleResolver){
  registry.registerStyleResolver("QWCityProof", ()=>qwCityProofStyleDef);
}

export {
  setupWidgetStyles,

}