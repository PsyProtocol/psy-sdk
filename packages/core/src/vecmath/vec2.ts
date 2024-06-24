import { TNodeAnchor } from "../anchor"
import { ISize, IVec2, TGenericVec2 } from "../types/geo"

function add(a: IVec2, b: IVec2): IVec2 {
  return {
    x: a.x + b.x,
    y: a.y + b.y,
  }
}
function sub(a: IVec2, b: IVec2): IVec2 {
  return {
    x: a.x - b.x,
    y: a.y - b.y,
  }
}
function midpoint(a: IVec2, b: IVec2): IVec2 {
  return {
    x: (a.x + b.x) / 2,
    y: (a.y + b.y) / 2,
  }
}
function dot(a: IVec2, b: IVec2): IVec2 {
  return {
    x: a.x * b.x,
    y: a.y * b.y,
  }
}
function mulScalar(p: IVec2, scalar: number): IVec2 {
  return {
    x: p.x * scalar,
    y: p.y * scalar
  }
}

function normalize(p: IVec2): IVec2 {
  const length = Math.sqrt(p.x * p.x + p.y * p.y);
  return {
    x: p.x / length,
    y: p.y / length,
  }
}

function distance(a: IVec2, b: IVec2): number {
  return Math.sqrt((a.x - b.x)*(a.x - b.x) + (a.y - b.y)*(a.y - b.y));
}
function radialOffset(origin: IVec2, angle: number, distance: number): IVec2 {
  return {
    x: origin.x + Math.cos(angle) * distance,
    y: origin.y + Math.sin(angle) * distance,
  }
}

function anchorUV(anchor: TNodeAnchor): IVec2 {
  const horizontal = ((anchor>>4)-1)*0.5
  const vertical =  (((anchor&0b111)>>1)-1)*0.5;
  return {x: horizontal, y: vertical};
}

function toSize(p: TGenericVec2): ISize {
  const v2 = from(p);
  return {width: v2.x, height: v2.y};
}
function from(x: number) : IVec2;
function from(p: TGenericVec2) : IVec2;
function from(x: number, y: number) : IVec2;
function from(p: TGenericVec2 | number, y?: number) : IVec2 {
  if(typeof y === 'number'){
    return {x: p as number, y: y};
  }else if(typeof p == 'number'){
    return {x: p as number, y: p as number};
  }else if(Array.isArray(p)){
    return {x: p[0], y: p[1]};
  }else if(typeof (p as ISize).width === 'number'){
    return {x: (p as ISize).width, y: (p as ISize).height};
  }else{
    return p as IVec2;
  }
}


const point = from;
const p = from;

function vec2Constructor(x: number) : IVec2;
function vec2Constructor(p: TGenericVec2) : IVec2;
function vec2Constructor(x: number, y: number) : IVec2;
function vec2Constructor(p: TGenericVec2 | number, y?: number) : IVec2 {
  if(typeof y === 'number'){
    return {x: p as number, y: y};
  }else if(typeof p == 'number'){
    return {x: p as number, y: p as number};
  }else if(Array.isArray(p)){
    return {x: p[0], y: p[1]};
  }else if(typeof (p as ISize).width === 'number'){
    return {x: (p as ISize).width, y: (p as ISize).height};
  }else{
    return p as IVec2;
  }
}

const v2m = Object.assign(vec2Constructor, {
  add,
  sub,
  midpoint,
  dot,
  mulScalar,
  normalize,
  distance,
  radialOffset,
  anchorUV,
  toSize,
  from,
  point,
  p,
});

export {
  add,
  sub,
  midpoint,
  dot,
  mulScalar,
  normalize,
  distance,
  radialOffset,
  anchorUV,
  toSize,
  from,
  point,
  p,
}

export default v2m;