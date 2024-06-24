import { TNodeAnchor, TRectSide } from '../anchor';
import { IAnchoredBoundingBox, IBoundingBox, IBoundingBoxCorners, ISize, IVec2 } from '../types/geo';
import * as v2m from './vec2';


function getCorners(bbox: IBoundingBox): IBoundingBoxCorners {
  return {
    topLeft: {
      x: bbox.center.x - bbox.size.width / 2,
      y: bbox.center.y - bbox.size.height / 2,
    },
    bottomRight: {
      x: bbox.center.x + bbox.size.width / 2,
      y: bbox.center.y + bbox.size.height / 2,
    }
  }
}
function bboxForPoints(points: IVec2[]): IBoundingBox {
  if (points.length === 0) {
    return { center: { x: 0, y: 0 }, size: { width: 0, height: 0 } };
  }
  let minX = points[0].x;
  let minY = points[0].y;
  let maxX = minX;
  let maxY = maxX;
  for (let i = 1, l = points.length; i < l; i++) {
    const x = points[i].x;
    const y = points[i].y;
    if (x < minX) {
      minX = x;
    }
    if (x > maxX) {
      maxX = x;
    }
    if (y < minY) {
      minY = y;
    }
    if (y > maxY) {
      maxY = y;
    }
  }
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  const width = maxX - minX;
  const height = maxY - minY;
  return {
    center: { x: centerX, y: centerY },
    size: { width, height }
  }
}
function bboxForRects(bboxes: IBoundingBox[]): IBoundingBox {
  if (bboxes.length === 0) {
    return { center: { x: 0, y: 0 }, size: { width: 0, height: 0 } };
  }
  const points: IVec2[] = [];
  for (let i = 0, l = bboxes.length; i < l; i++) {
    const corners = getCorners(bboxes[i]);
    points.push(corners.topLeft);
    points.push(corners.bottomRight);
  }
  return bboxForPoints(points);
}

function fromCorners(corners: IBoundingBoxCorners): IBoundingBox;
function fromCorners(topLeft: IVec2, bottomRight: IVec2): IBoundingBox;
function fromCorners(topLeftX: number, topLeftY: number, bottomRightX: number, bottomRightY: number): IBoundingBox;
function fromCorners(
  cornersOrTopLeftOrTopLeftX: IBoundingBoxCorners | IVec2 | number,
  bottomRightOrTopLeftY?: IVec2 | number,
  bottomRightX?: number,
  bottomRightY?: number,
): IBoundingBox {
  if (typeof cornersOrTopLeftOrTopLeftX === 'object') {
    if (typeof (cornersOrTopLeftOrTopLeftX as IVec2).x === 'number') {
      return {
        center: v2m.midpoint(cornersOrTopLeftOrTopLeftX as IVec2, bottomRightOrTopLeftY as IVec2),
        size: v2m.toSize(v2m.sub(bottomRightOrTopLeftY as IVec2, cornersOrTopLeftOrTopLeftX as IVec2)),
      }
    } else {
      const corners = cornersOrTopLeftOrTopLeftX as IBoundingBoxCorners;
      return {
        center: v2m.add(corners.topLeft, v2m.mulScalar(v2m.sub(corners.bottomRight, corners.topLeft), 0.5)),
        size: v2m.toSize(v2m.sub(corners.bottomRight, corners.topLeft)),
      }
    }
  } else {

    const topLeftX = cornersOrTopLeftOrTopLeftX as number;
    const topLeftY = bottomRightOrTopLeftY as number;
    const centerX = (topLeftX + bottomRightX!) / 2;
    const centerY = (topLeftY + bottomRightY!) / 2;
    const width = bottomRightX! - topLeftX;
    const height = bottomRightY! - topLeftY;

    return {
      center: { x: centerX, y: centerY },
      size: { width, height }
    }
  }
}
function centerAnchoredCore(bbox: IAnchoredBoundingBox): IBoundingBox {
  const centerPoint = v2m.add(bbox.point, v2m.dot(v2m.anchorUV(bbox.anchor), v2m.p(bbox.size)));

  return {
    center: centerPoint,
    size: bbox.size,
  }
}
function anchorAt(anchor: TNodeAnchor, point: IVec2, size: ISize): IBoundingBox;
function anchorAt(bbox: IAnchoredBoundingBox): IBoundingBox;
function anchorAt(bboxOrAnchor: IAnchoredBoundingBox | TNodeAnchor, point?: IVec2, size?: ISize): IBoundingBox {
  if (typeof bboxOrAnchor === 'number') {
    return centerAnchoredCore({ anchor: bboxOrAnchor, point: point!, size: size! });
  } else {
    return centerAnchoredCore(bboxOrAnchor as IAnchoredBoundingBox);
  }
}
function getAnchorPosition(bbox: IBoundingBox, anchor: TNodeAnchor): IVec2 {
  return anchorAt(anchor, bbox.center, bbox.size).center;
}
function swapAnchor(bbox: IBoundingBox, newAnchor: TNodeAnchor): IBoundingBox {
  return anchorAt(newAnchor, bbox.center, bbox.size);
}


function rectAt(center: IVec2, size: ISize): IBoundingBox {
  return {
    center,
    size,
  }
}

function distanceToCenter(bbox: IBoundingBox, point: IVec2): number {
  return v2m.distance(bbox.center, point);
}

function distanceToAnchor(bbox: IBoundingBox, anchor: TNodeAnchor, point: IVec2): number {
  return distanceToCenter(anchorAt(anchor, bbox.center, bbox.size), point);
}

function distanceToSide(bbox: IBoundingBox, side: TRectSide, point: IVec2): number {
  return distanceToAnchor(bbox, side, point);
}

function innerRadius(size: ISize) {
  return Math.max(size.width, size.height) / 2;
}

function radius(size: ISize) {
  return Math.sqrt(size.width * size.width + size.height * size.height) / 2;
}

function pad(bbox: IBoundingBox, padding: number): IBoundingBox {
  return {
    center: bbox.center,
    size: {
      width: bbox.size.width + padding * 2,
      height: bbox.size.height + padding * 2,
    },
  }
}

// computes the point which is offset from the anchor point
function getOffsetPointFromAnchor(bbox: IBoundingBox, anchor: TNodeAnchor, offset: IVec2): IVec2 {
  return v2m.add(getAnchorPosition(bbox, anchor), offset);
}

function getOffsetBBoxFromAnchor(sourceBBox: IBoundingBox, sourceAnchor: TNodeAnchor, targetSize: ISize, targetAnchor: TNodeAnchor, targetOffset: IVec2): IBoundingBox {
  const targetCenter = getOffsetPointFromAnchor(anchorAt(targetAnchor, sourceBBox.center, targetSize), targetAnchor, targetOffset);
  return anchorAt(sourceAnchor, targetCenter, targetSize);
}


function bboxConstructor(domRect: DOMRect): IBoundingBox;
function bboxConstructor(centerPoint: IVec2, size: ISize): IBoundingBox;
function bboxConstructor(centerX: number, centerY: number, size: ISize): IBoundingBox;
function bboxConstructor(centerX: number, centerY: number, width: number, height: number): IBoundingBox;
function bboxConstructor(domRectOrCenterPointOrX: DOMRect | IVec2 | number, sizeOrY?: ISize | number, sizeOrWidth?: ISize | number, height?: number): IBoundingBox {
  if(typeof sizeOrY === 'undefined' && typeof sizeOrWidth === 'undefined' && typeof height === 'undefined'){
    const domRect = domRectOrCenterPointOrX as DOMRect;
    return {
      center: {x: domRect.x + domRect.width/2, y: domRect.y + domRect.height/2},
      size: {width: domRect.width, height: domRect.height},
    }
  }else if(typeof domRectOrCenterPointOrX === 'object' && typeof sizeOrY === 'object'){
    return {
      center: domRectOrCenterPointOrX as IVec2,
      size: sizeOrY as ISize,
    }
  }else if(typeof domRectOrCenterPointOrX  === 'number' && typeof sizeOrY === 'number' && typeof sizeOrWidth === 'object'){
    return {
      center: {x: domRectOrCenterPointOrX , y: sizeOrY},
      size: sizeOrWidth as ISize,
    }
  }else if(typeof domRectOrCenterPointOrX  === 'number' && typeof sizeOrY === 'number' && typeof sizeOrWidth === 'number' && typeof height === 'number'){
    return {
      center: {x: domRectOrCenterPointOrX , y: sizeOrY},
      size: {width: sizeOrWidth, height},
    }
  }else{
    throw new Error('Invalid arguments passed to bbox constructor');
  }
}

const bbox = Object.assign(bboxConstructor, {
  getCorners,
  bboxForPoints,
  bboxForRects,
  fromCorners,
  centerAnchoredCore,
  anchorAt,
  swapAnchor,
  rectAt,
  distanceToCenter,
  distanceToAnchor,
  distanceToSide,
  innerRadius,
  radius,
  pad,
  getOffsetPointFromAnchor,
  getOffsetBBoxFromAnchor,
});

export {
  getCorners,
  bboxForPoints,
  bboxForRects,
  fromCorners,
  centerAnchoredCore,
  anchorAt,
  swapAnchor,
  rectAt,
  distanceToCenter,
  distanceToAnchor,
  distanceToSide,
  innerRadius,
  radius,
  pad,
  getOffsetPointFromAnchor,
  getOffsetBBoxFromAnchor,


  bbox,
};


export default bbox;
