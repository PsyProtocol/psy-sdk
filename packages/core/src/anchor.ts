import { IVec2 } from "./types/geo";

const VerticalAnchor = {
  Top: 0b000001,
  Center: 0b000010,
  Bottom: 0b000100,
};
const HorizontalAnchor = {
  Left: 0b001000,
  Center: 0b010000,
  Right: 0b100000,
};
type TNodeAnchor = 9 | 10 | 12 | 17 | 18 | 20 | 33 | 34 | 36;
const TopLeft = (VerticalAnchor.Top | HorizontalAnchor.Left) as TNodeAnchor;
const TopCenter = (VerticalAnchor.Top | HorizontalAnchor.Center) as TNodeAnchor;
const TopRight = (VerticalAnchor.Top | HorizontalAnchor.Right) as TNodeAnchor;
const CenterLeft = (VerticalAnchor.Center | HorizontalAnchor.Left) as TNodeAnchor;
const Center = (VerticalAnchor.Center | HorizontalAnchor.Center) as TNodeAnchor;
const CenterRight = (VerticalAnchor.Center | HorizontalAnchor.Right) as TNodeAnchor;
const BottomLeft = (VerticalAnchor.Bottom | HorizontalAnchor.Left) as TNodeAnchor;
const BottomCenter = (VerticalAnchor.Bottom | HorizontalAnchor.Center) as TNodeAnchor;
const BottomRight = (VerticalAnchor.Bottom | HorizontalAnchor.Right) as TNodeAnchor;
type TVerticalAnchor = 1 | 2 | 4;
type THorizontalAnchor = 8 | 16 | 32;

type TRectSide = 17 | 34 | 20 | 10;
const RectSide = {
  Top: TopCenter as TRectSide,
  Right: CenterRight as TRectSide,
  Bottom: BottomCenter as TRectSide,
  Left: CenterLeft as TRectSide,
};

const OpposideAnchors: Record<TNodeAnchor, TNodeAnchor> = {
  [TopLeft]: BottomRight,
  [TopCenter]: BottomCenter,
  [TopRight]: BottomLeft,
  [CenterLeft]: CenterRight,
  [Center]: Center,
  [CenterRight]: CenterLeft,
  [BottomLeft]: TopRight,
  [BottomCenter]: TopCenter,
  [BottomRight]: TopLeft,
} as Record<TNodeAnchor, TNodeAnchor>;


const NodeAnchorIndexed: TNodeAnchor[] = [
  TopLeft,
  TopCenter,
  TopRight,
  CenterLeft,
  Center,
  CenterRight,
  BottomLeft,
  BottomCenter,
  BottomRight,
];
const NodeAnchor = {
  TopLeft: TopLeft as TNodeAnchor,
  TopCenter: TopCenter as TNodeAnchor,
  TopRight: TopRight as TNodeAnchor,
  CenterLeft: CenterLeft as TNodeAnchor,
  Center: Center as TNodeAnchor,
  CenterRight: CenterRight as TNodeAnchor,
  BottomLeft: BottomLeft as TNodeAnchor,
  BottomCenter: BottomCenter as TNodeAnchor,
  BottomRight: BottomRight as TNodeAnchor,
};

console.log(JSON.stringify(NodeAnchor));

export type {
  TNodeAnchor,
  TRectSide,
  TVerticalAnchor,
  THorizontalAnchor,
};
export {
  NodeAnchor,
  HorizontalAnchor,
  VerticalAnchor,
  NodeAnchorIndexed,
  RectSide,
  OpposideAnchors,
}