import { TNodeAnchor, TRectSide } from "../anchor";

interface IVec2 {
  x: number;
  y: number;
}

interface ISize {
  width: number;
  height: number;
}

interface IBoundingBox {
  center: IVec2;
  size: ISize;
}

interface IAnchoredBoundingBox {
  anchor: TNodeAnchor;
  point: IVec2;
  size: ISize;
}

interface IAnchoredPoint {
  point: IVec2;
  anchor: TNodeAnchor;
}


interface IAnchorRef {
  id: string;
  anchor: TNodeAnchor;
  offset?: IVec2;
}

interface IRadialOffsetPoint {
  origin: IVec2;
  angle: number;
  distance: number;
}


interface IRadialRectOffsetPoint {
  origin: IBoundingBox;
  side: TRectSide;
  distanceToSide: number;
  angle: number;
}

interface IBoundingBoxCorners {
  topLeft: IVec2;
  bottomRight: IVec2;
}
type TGenericVec2 = IVec2 | ISize | number[] | [number, number];

export type {
  IVec2,
  ISize,
  IBoundingBox,
  IAnchoredBoundingBox,
  IRadialRectOffsetPoint,
  IAnchoredPoint,
  IRadialOffsetPoint,
  IBoundingBoxCorners,
  TGenericVec2,
  IAnchorRef,
}