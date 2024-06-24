import { TNodeAnchor } from "../anchor";
import { IAnchorRef, ISize, IVec2 } from "./geo";

interface IQWidgetSerializedCore<C, S> {
  id: string;
  type: string;
  position: IVec2;
  size: ISize;
  config: C;
  state: S;
}

interface IQWidgetSerialized<C, S> extends IQWidgetSerializedCore<C, S> {
  children?: IQWidgetSerialized<any, any>[];
}

interface ISimpleEdgeStyle {
  color?: string;
  strokeWidth?: number;
  strokeDasharray?: string;
  className?: string;
}

interface IResolvedSimpleEdgeDefinition extends ISimpleEdgeStyle {
  id?: string;
  from: IVec2;
  to: IVec2;
}

interface ISimpleEdgeDef extends ISimpleEdgeStyle {
  id?: string;
  fromRef?: IAnchorRef;
  toRef?: IAnchorRef;
  from?: IVec2;
  to?: IVec2;
}

export type {
  IQWidgetSerializedCore,
  IQWidgetSerialized,
  ISimpleEdgeStyle,
  IResolvedSimpleEdgeDefinition,
  ISimpleEdgeDef,
}