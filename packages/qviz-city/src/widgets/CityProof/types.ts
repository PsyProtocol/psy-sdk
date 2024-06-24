import { ISize, ITextElementHelper } from "@qstudio/core";
import { CityProofStateType } from ".";

interface IQVCityProofStyleDef {
  states: {
    hidden: string;
    waiting: string;
    proving: string;
    proved: string; 
  },
  refLink: string;
  base: string;
  outerGroup: string;
  borderRect: string;
  label: string;
  statusGroup: string;
  statusRect: string;
  statusText: string;
  styleConfig: ICircuitIconStyle;
  iconRoot: string;
}

interface ICityProofStyleState {
  baseClassName: string;
  labelText: string;
  statusText: string;
}
interface IQVCityProofNodeElems {
  base: SVGGElement;
  outerGroup: SVGGElement;
  borderRect: SVGRectElement;
  label: ITextElementHelper;
  statusGroup: SVGGElement;
  statusRect: SVGRectElement;
  statusText: ITextElementHelper;
}



interface ICircuitIconDef {
  g: SVGGElement;
  width: number;
  height: number;
}
interface ICircuitIconStyle {
  fillColorClass: string;
  strokeColorClass: string;
  gClass: string;
}


enum ProofWidgetStyleVariant {
  TextOnly = 0,
  Standard = 1,
  Aggregate = 2,
  BigAggregate = 3,
}
interface IProofIconHelper {
  setState(state: CityProofStateType): void;
  getGroup(): SVGGElement;
  getSize(): ISize;
}
interface ICityProofInfoStore {
  getProvingTime(jobId: string): number;
  getProofIconForJob(jobId: string, styleDef: IQVCityProofStyleDef): IProofIconHelper;
  getProofWidgetVariantForJob(jobId: string): ProofWidgetStyleVariant;
}

export {ProofWidgetStyleVariant};

export type {
  IQVCityProofStyleDef,
  ICityProofStyleState,
  IQVCityProofNodeElems,
  ICircuitIconDef,
  ICircuitIconStyle,
  IProofIconHelper,
  ICityProofInfoStore,
}