import {IBoundingBox, IQSRenderContext, IQVizStyleResolver, ITextElementHelper, QWidget, multilineTextGroupV3, simpleStateDiff, } from '@qstudio/core';
import { makeSVGElemAttributes, makeSVGElement } from '@qstudio/qsvg';
import { getCircuitNameForJobIdHex, getCircuitWidgetNameForJobIdHex } from '@qstudio/city-block';
import { ICityProofInfoStore, ICityProofStyleState, IQVCityProofNodeElems, IQVCityProofStyleDef } from './types';
import { QWCityProofTotalHeight, getNodeElems } from './render';
import { globalProofInfoStore } from './infoStore';

const QWCityProofRectWidth = 228;
enum CityProofStateType {
  Hidden = "hidden",
  Waiting = "waiting",
  Proving = "proving",
  Proved = "proved",
}
const StateStatusMessages: Record<CityProofStateType, string> = {
  [CityProofStateType.Hidden]: "",
  [CityProofStateType.Waiting]: "Waiting for Prover...",
  [CityProofStateType.Proving]: "Proving...",
  [CityProofStateType.Proved]: "Proved",
};
const WIDGET_TYPE_ID = "QWCityProof";

function nameHelper(x: string){
  if(x.startsWith("[AGG]")){
    return x.slice(5).trim()+"\n[Aggregate]";
  }else{
    return x;
  }
}
function getStyleState(widget: QWCityProof, styleDef: IQVCityProofStyleDef): ICityProofStyleState{
  const baseClassName = styleDef.states[widget.state.stateType];
  const labelText = nameHelper(getCircuitWidgetNameForJobIdHex(widget.config.jobId));
  const statusText = StateStatusMessages[widget.state.stateType];
  return {
    baseClassName,
    labelText,
    statusText,
  };
}

interface IQWCityProofConfig {
  className?: string;
  jobId: string;
  isRef?: boolean

}
interface IQWCityProofState {
  stateType: CityProofStateType;
}

type IQWCityProofStatePatch = Partial<IQWCityProofState>;

class QWCityProof extends QWidget<IQWCityProofConfig, IQWCityProofState, IQWCityProofStatePatch> {
  infoStore: ICityProofInfoStore = globalProofInfoStore;

  elems?: IQVCityProofNodeElems;
  styleDef?: IQVCityProofStyleDef;
  getChildren(): QWidget<any, any, any>[] {
    return [];
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    return {
      center: this.position,
      size: {
        width: QWCityProofRectWidth,
        height: QWCityProofTotalHeight,
      },
    };
  }
  getWidgetType(): string {
    return WIDGET_TYPE_ID;
  }
  
  getDefaultState(): IQWCityProofState {
    return {
      stateType: CityProofStateType.Waiting,
    };
  }
  renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const styleDef = context.manager.getStyleDef<IQVCityProofStyleDef, IQWCityProofConfig>(this.getWidgetType(), this.config);
    this.styleDef = styleDef;
    const styleState = getStyleState(this, styleDef);
    const icon = this.infoStore.getProofIconForJob(this.config.jobId, styleDef);
    const baseB = getNodeElems(context, styleDef, styleState, icon.getGroup(), icon.getSize(), this.infoStore.getProofWidgetVariantForJob(this.config.jobId));
    this.elems = baseB;
    container.appendChild(baseB.outerGroup);


    //baseB.label.setText(styleState.labelText);
    //baseB.statusText.setText(styleState.statusText);
    const baseClassNames = [styleDef.base, styleState.baseClassName];
    if(this.config.isRef){
      baseClassNames.push(styleDef.refLink);
    }
    baseB.base.setAttribute("class", baseClassNames.join(" "));
    container.removeChild(baseB.outerGroup);
    return baseB.outerGroup;
  }
  applyStateUpdate(stateUpdate: Partial<IQWCityProofState>): boolean {
    const {newState, diff, changed} = simpleStateDiff(this.state, stateUpdate);
    if(!changed || !this.elems && !this.styleDef){
      return false;
    }
    this.state = newState;
    if(typeof diff.stateType !== 'undefined'){
      const styleDef = this.styleDef!;

      const styleState = getStyleState(this, styleDef);
      this.elems!.base.setAttribute("class", styleDef.base + " " + styleState.baseClassName);
      this.elems!.statusText.setText(styleState.statusText);
    }
    return false;
  }
}

export type {
  IQWCityProofState,
}
export {
  QWCityProof,
  CityProofStateType,
}