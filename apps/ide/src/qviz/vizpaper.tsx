/*
import { SVG } from '@svgdotjs/svg.js'
import '@svgdotjs/svg.panzoom.js'
import { ISize } from '@qstudio/core';
import type {Svg} from '@svgdotjs/svg.js';

class QEDVizPaper {
  svg: Svg;
  root: SVGGElement;
  constructor(container: HTMLElement, size: ISize){
    this.svg = SVG()
      .addTo(container)
      .size(size.width, size.height)
      .viewbox(`0 0 ${size.width} ${size.height}`)
      .panZoom();
    this.root = this.svg.group().node;
  }
  
  clear() {
    while(this.root.lastChild){
      this.root.removeChild(this.root.lastChild);
    }
    this.root = this.svg.group().node;
  }


  getRoot(): SVGGElement {
    return this.root;
  }

  resizeToFit(padding = 50){
    const parentBounds = this.svg.node.parentElement?.getBoundingClientRect();

    this.svg.size(parentBounds?.width, parentBounds?.height);
    const realBBox = this.svg.node.getBBox();
    const bbox = {
      x: realBBox.x-padding,
      y: realBBox.y-padding,
      width: realBBox.width+padding*2,
      height: realBBox.height+padding*2
    };
    const svgBounding = this.svg.node.getBoundingClientRect();
    const aspectBBox = bbox.width/bbox.height;
    const aspectSvg = svgBounding.width/svgBounding.height;
    const viewMin = {x: 0, y: 0};
    const viewMax = {x: 0, y: 0};
    if(aspectSvg>=aspectBBox){
      viewMin.y = bbox.y;
      viewMax.y = bbox.y+bbox.height;
      const center = bbox.x+bbox.width/2;
      const halfWidth = bbox.height*aspectSvg/2;
      viewMin.x = center-halfWidth;
      viewMax.x = center+halfWidth;
    }else{
      viewMin.x = bbox.x;
      viewMax.x = bbox.x+bbox.width;
      const center = bbox.y+bbox.height/2;
      const halfHeight = bbox.width/aspectSvg/2;
      viewMin.y = center-halfHeight;
      viewMax.y = center+halfHeight;
    }
    this.svg.viewbox(viewMin.x, viewMin.y, viewMax.x-viewMin.x, viewMax.y-viewMin.y);

  }
  dispose(){
    this.svg.remove();
  }
}

export {
  QEDVizPaper,

}*/

export const test123 = 123;