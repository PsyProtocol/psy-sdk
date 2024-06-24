import { TRectSide } from "../anchor";
import { IBoundingBox, ISize } from "./geo";

interface ITreeJunctionLayout {
  direction: TRectSide;
  siblingSpacing: number;
  levelSpacing: number;
  parentAnchor: TRectSide;
  childAnchor: TRectSide;
  edgeClassName?: string;
}

interface ITreeNodeLayout extends ITreeJunctionLayout{
  siblingAxisAnchor: TRectSide;
}

interface ISimpleLayoutResult {
  children: IBoundingBox[];
  container: IBoundingBox;
}



interface ISimpleLayoutResultFinal {
  children: IBoundingBox[];
  node: IBoundingBox;
  container: IBoundingBox;
}


export type {
  ITreeJunctionLayout,
  ITreeNodeLayout,
  ISimpleLayoutResult,
  ISimpleLayoutResultFinal,
}