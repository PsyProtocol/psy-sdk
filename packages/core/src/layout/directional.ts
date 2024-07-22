import {
  HorizontalAnchor,
  OpposideAnchors,
  RectSide,
  TNodeAnchor,
  TRectSide,
  TVerticalAnchor,
  VerticalAnchor,
} from "../anchor";
import { IBoundingBox, ISize, IVec2 } from "../types/geo";
import { ISimpleLayoutResult, ISimpleLayoutResultFinal, ITreeJunctionLayout } from "../types/layout";
import { v2m } from "../vecmath";
import bbox, { bboxForRects, getOffsetBBoxFromAnchor } from "../vecmath/bbox";

function layoutHorizontal(
  sizes: ISize[],
  spacing: number,
  inAxisAnchor: TNodeAnchor
): ISimpleLayoutResult {
  if (sizes.length === 0) {
    return {
      children: [],
      container: {
        center: { x: 0, y: 0 },
        size: { width: 0, height: 0 },
      },
    };
  } else if (sizes.length === 1) {
    return {
      children: [{ center: { x: 0, y: 0 }, size: sizes[0] }],
      container: { center: { x: 0, y: 0 }, size: sizes[0] },
    };
  }

  const axisAnchor = inAxisAnchor & 0b111;

  let totalWidth = 0;
  let totalHeight = 0;
  const count = sizes.length;
  for (let i = 0; i < count; i++) {
    totalWidth += sizes[i].width;
    totalHeight = Math.max(totalHeight, sizes[i].height);
  }
  totalWidth += spacing * (count - 1);

  const childBoxes: IBoundingBox[] = [];
  let currentWidth = 0;
  for (let i = 0; i < count; i++) {
    const xOffset = currentWidth + sizes[i].width / 2;
    currentWidth += sizes[i].width + spacing;
    const y =
      axisAnchor === VerticalAnchor.Top
        ? (sizes[i].height - totalHeight) / 2
        : axisAnchor === VerticalAnchor.Bottom
        ? (totalHeight - sizes[i].height) / 2
        : 0;
    childBoxes.push({
      center: { x: xOffset, y },
      size: sizes[i],
    });
  }

  return {
    container: {
      center: { x: -totalWidth/2, y: 0 },
      size: { width: totalWidth, height: totalHeight },
    },
    children: childBoxes,
  };
}

function layoutVertical(
  sizes: ISize[],
  spacing: number,
  inAxisAnchor: TNodeAnchor
): ISimpleLayoutResult {
  if (sizes.length === 0) {
    return {
      children: [],
      container: {
        center: { x: 0, y: 0 },
        size: { width: 0, height: 0 },
      },
    };
  } else if (sizes.length === 1) {
    return {
      children: [{ center: { x: 0, y: 0 }, size: sizes[0] }],
      container: { center: { x: 0, y: 0 }, size: sizes[0] },
    };
  }

  const axisAnchor = inAxisAnchor >> 3;

  let totalWidth = 0;
  let totalHeight = 0;
  const count = sizes.length;
  for (let i = 0; i < count; i++) {
    totalHeight += sizes[i].height;
    totalWidth = Math.max(totalWidth, sizes[i].width);
  }
  totalHeight += spacing * (count - 1);

  const childBoxes: IBoundingBox[] = [];
  let currentHeight = 0;
  for (let i = 0; i < count; i++) {
    const yOffset = currentHeight + sizes[i].height / 2;
    const x =
      axisAnchor === HorizontalAnchor.Left
        ? (sizes[i].width - totalWidth) / 2
        : axisAnchor === HorizontalAnchor.Right
        ? (totalWidth - sizes[i].width) / 2
        : 0;
    childBoxes.push({
      center: { x, y: yOffset - totalHeight / 2 },
      size: sizes[i],
    });
  }

  return {
    container: {
      center: { x: 0, y: 0 },
      size: { width: totalWidth, height: totalHeight },
    },
    children: childBoxes,
  };
}

function getLevelSpacingForLayoutConfig(layoutConfig: ITreeJunctionLayout): IVec2 {
  const amount = layoutConfig.levelSpacing;
  if(layoutConfig.direction === RectSide.Bottom){
    return {x: 0, y: -amount};
  }else if(layoutConfig.direction === RectSide.Top){
    return {x: 0, y: amount};
  }else if(layoutConfig.direction === RectSide.Left){
    return {x: amount, y: 0};
  }else if(layoutConfig.direction === RectSide.Right){
    return {x: -amount, y: 0};
  }else{
    return {x: 0, y: 0};
  }
}
function layoutFinalNodeOld(
  nodeSize: ISize,
  layoutConfig: ITreeJunctionLayout,
  childrenResult: ISimpleLayoutResult
) : ISimpleLayoutResultFinal{
  const nodeBBox = {center: {x:0, y:0}, size: nodeSize};
  let spacing = getLevelSpacingForLayoutConfig(layoutConfig);
  const newContainer = getOffsetBBoxFromAnchor(nodeBBox,layoutConfig.parentAnchor, childrenResult.container.size, OpposideAnchors[layoutConfig.direction], spacing);
  let conOffset = v2m.sub(childrenResult.container.center, newContainer.center);

  
  const newBBox = bboxForRects([nodeBBox, newContainer]);
  nodeBBox.center = v2m.sub( newBBox.center, nodeBBox.center);
  conOffset = v2m.add(conOffset, nodeBBox.center);

  const newChildren = childrenResult.children.map(child => {
    return {
      ...child,
      center: v2m.add(child.center, conOffset)
    }
  });
  return {
    children: newChildren,
    container: bboxForRects([nodeBBox, newContainer]),
    node: nodeBBox,
  }
}


function layoutFinalNodeOld2(
  nodeSize: ISize,
  layoutConfig: ITreeJunctionLayout,
  childrenResult: ISimpleLayoutResult
) : ISimpleLayoutResultFinal{
  const nodeBBox = {center: {x:0, y:0}, size: nodeSize};
  let spacing = getLevelSpacingForLayoutConfig(layoutConfig);
  const newContainer = getOffsetBBoxFromAnchor(nodeBBox,layoutConfig.parentAnchor, childrenResult.container.size, OpposideAnchors[layoutConfig.direction], spacing);
  let conOffset = v2m.sub(childrenResult.container.center, newContainer.center);

  
  //const newBBox = bboxForRects([nodeBBox, newContainer]);
  //newBBox.center.x = 0;
  //nodeBBox.center = v2m.sub( newBBox.center, nodeBBox.center);
  nodeBBox.center.x = 0;
  //conOffset = v2m.add(conOffset, nodeBBox.center);

  const newChildren = childrenResult.children.map(child => {
    return {
      ...child,
      center: v2m.add(child.center, conOffset)
    }
  });
  return {
    children: newChildren,
    container: bboxForRects([nodeBBox, newContainer]),
    node: nodeBBox,
  }
}
function layoutFinalNode(
  nodeSize: ISize,
  layoutConfig: ITreeJunctionLayout,
  childrenResult: ISimpleLayoutResult
) : ISimpleLayoutResultFinal{
  const nodeBBox = {center: {x:0, y:0}, size: nodeSize};
  let spacing = getLevelSpacingForLayoutConfig(layoutConfig);
  const newContainer = getOffsetBBoxFromAnchor(nodeBBox,layoutConfig.parentAnchor, childrenResult.container.size, OpposideAnchors[layoutConfig.direction], spacing);

  nodeBBox.center = newContainer.center;
  let conOffset = v2m.sub(childrenResult.container.center, newContainer.center);

  
  const newBBox = bboxForRects([nodeBBox, newContainer]);
  //nodeBBox.center = v2m.sub( newBBox.center, nodeBBox.center);
  //conOffset = v2m.add(conOffset, newBBox.center);

  const newChildren = childrenResult.children.map(child => {
    return {
      ...child,
     center: v2m.add(child.center, conOffset)
    }
  });
  return {
    children: newChildren,
    container: newBBox,
    node: nodeBBox,
  }
}
function layoutDirectional(
  node: ISize,
  children: ISize[],
  layoutConfig: ITreeJunctionLayout
) {
  const results =
    layoutConfig.direction === RectSide.Bottom ||
    layoutConfig.direction === RectSide.Top
      ? layoutHorizontal(
          children,
          layoutConfig.siblingSpacing,
          layoutConfig.childAnchor
        )
      : layoutVertical(
          children,
          layoutConfig.siblingSpacing,
          layoutConfig.childAnchor
        );
  return layoutFinalNode(node, layoutConfig, results);
}

export { layoutHorizontal, layoutVertical, layoutDirectional };
