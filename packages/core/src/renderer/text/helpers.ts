
import { HorizontalAnchor, NodeAnchor, TNodeAnchor, VerticalAnchor } from "../../anchor";
import { IBoundingBox, ISize, IVec2 } from "../../types/geo";
interface ITextElementHelper {
  g: SVGGraphicsElement;
  setText: (str: string) => void;
}

function alignBoundingBox(bbox: IBoundingBox, anchor: TNodeAnchor): IVec2{
  let x = bbox.center.x;
  let y = bbox.center.y;
  if(anchor & HorizontalAnchor.Center){
    x += bbox.size.width / 2;
  }else if(anchor & HorizontalAnchor.Right){
    x += bbox.size.width;
  }
  
  if(anchor & VerticalAnchor.Center){
    y -= bbox.size.height / 2;
  }else if(anchor & VerticalAnchor.Bottom){
    y += bbox.size.height;
  }

  return {x, y};
  
}
const SVG_RICH_TEXT_BLOCK_CONTROL = String.fromCharCode(1337);
const SVG_RICH_TEXT_STYLE_END_CONTROL = String.fromCharCode(1338);
function decomposeLine(line: string, baseExtraProps: any = {}){
  if(line.indexOf(SVG_RICH_TEXT_BLOCK_CONTROL)===-1){
    return [createGraphicsElement("tspan",{textContent: line})];
  }
  return line.split(SVG_RICH_TEXT_BLOCK_CONTROL).filter(x=>x).map(chunk=>{
    const styleEnd = chunk.indexOf(SVG_RICH_TEXT_STYLE_END_CONTROL);

    if(styleEnd!==-1){
      let extraProps = {};
      try {
        extraProps = {...baseExtraProps, ...(JSON.parse(chunk.substring(0,styleEnd)))};
        return createGraphicsElement("tspan",{...extraProps,textContent: chunk.substring(styleEnd+1)});

      }catch(err){
        console.error("ERROR Parsing style: ",err);
        
      }
    }
    return createGraphicsElement("tspan",{...baseExtraProps,textContent: line});
  })




}

function createGraphicsElement(tagName: string, props: any = {}): SVGGraphicsElement{
  const elem = document.createElementNS("http://www.w3.org/2000/svg", tagName);
  Object.keys(props).forEach(key => {
    if(key === 'textContent'){
      elem.textContent = props[key];
    }else{
      elem.setAttributeNS(null, key, props[key]);
    }
  });
  return elem as SVGGraphicsElement;
}
function createPathElement(startPoint: IVec2, endPoint: IVec2, props: any = {}): SVGPathElement{
  const path = createGraphicsElement("path", props);
  path.setAttributeNS(null, "d", `M ${startPoint.x} ${startPoint.y} L ${endPoint.x} ${endPoint.y}`);
  return path as SVGPathElement;
}

function createGroupElement(children: {tagName: string, props?: any}[], props: any = {}){
  const g = createGraphicsElement("g", props);
  children.forEach(child => {
    g.appendChild(createGraphicsElement(child.tagName, child.props));
  });
  return g;
}

function svgRichText(text: string, props: any = {}){
  return SVG_RICH_TEXT_BLOCK_CONTROL+JSON.stringify(props)+SVG_RICH_TEXT_STYLE_END_CONTROL+text+SVG_RICH_TEXT_BLOCK_CONTROL;
  
}
function parseLine(str: string){
  if(str.charCodeAt(0)===1337){
    const index = str.indexOf(String.fromCharCode(1337))

  }
}
function multilineTextGroup(position: IVec2, props: any = {}, multiplier=1.15, anchor = NodeAnchor.TopCenter){
  const g = createGroupElement([],{transform: `translate(${position.x},${position.y})`});
  const setText = (str: string)=>{
    while(g.lastChild){
      g.removeChild(g.lastChild);
    }
    const lines = str.split("\n").map(l=>decomposeLine(l));

    let y = 0;
    for(let line of lines){
      const elem = createGraphicsElement("text", {y, ...props});
      for(let tspan of line){
        elem.appendChild(tspan);
      }
      g.appendChild(elem);
      y+=elem.getBBox().height*multiplier;
    }
    const groupBox = g.getBBox();
    const newPoint = alignBoundingBox({
      center: {x: position.x-groupBox.width/2, y: position.y},
      size: {width: groupBox.width, height: groupBox.height/2}
    },anchor)
    if(anchor!==NodeAnchor.TopCenter)
    g.setAttributeNS(null, "transform", `translate(${newPoint.x}, ${newPoint.y})`);
  };
  return {
    g,
    setText,
  }
}

const HORIZONTAL_TEXT_ANCHOR = {
  [HorizontalAnchor.Left]: "start",
  [HorizontalAnchor.Center]: "middle",
  [HorizontalAnchor.Right]: "end",
};
const VERITCAL_TEXT_ANCHOR = {
  [VerticalAnchor.Top]: "text-after-edge",
  [VerticalAnchor.Center]: "middle",
  [VerticalAnchor.Bottom]: "text-before-edge",
};
function multilineTextGroupV2(position: IVec2, props: any = {}, multiplier=1.15, anchor = NodeAnchor.TopCenter, fitToContainer?: ISize){
  const g = createGroupElement([],{transform: `translate(${position.x},${position.y})`});
  const setText = (str: string)=>{
    let transformOffset = {x: position.x, y: position.y};
    while(g.lastChild){
      g.removeChild(g.lastChild);
    }
    const alignHorizText = HORIZONTAL_TEXT_ANCHOR[anchor&0b111000];
    const alignVertText = VERITCAL_TEXT_ANCHOR[anchor&0b111];
    const extraProps = {
      "text-anchor": alignHorizText,
      "dominant-baseline": alignVertText,
    };
    const lines = str.split("\n").map(l=>decomposeLine(l));
    let totalHeight = 0;
    let remainder = 0;
    const elemBboxes: {elem: SVGGraphicsElement, bbox: DOMRect}[] = [];
    for(let l=0;l<lines.length;l++){
      const line = lines[l];
      


      const elem = createGraphicsElement("text", {x:0, y: 0, ...props});
      elem.style.textAnchor = alignHorizText;
      elem.style.dominantBaseline = alignVertText;
      for(let tspan of line){
        elem.appendChild(tspan);
      }
      g.appendChild(elem);
      const bbox = elem.getBBox();

      elemBboxes.push({elem, bbox});
      totalHeight+=bbox.height*((l===0||l===(lines.length-1))?1:multiplier);
    }
    let heightSum = totalHeight;

    if(anchor&VerticalAnchor.Center){
      const middleInd = Math.floor(elemBboxes.length/2);
      heightSum = elemBboxes.slice(0,middleInd).map(x=>x.bbox.height).reduce((a,b)=>(a+b),0);
      if(elemBboxes.length&&(elemBboxes.length&1)===0){
        heightSum -=elemBboxes[middleInd-1].bbox.height/2;
      }
    }
    if(anchor&VerticalAnchor.Top){
      heightSum=-totalHeight;
    }else{
    }
    const startPos = totalHeight*((anchor&VerticalAnchor.Top)?-1:((anchor&VerticalAnchor.Center)?-0.5:0));;

    let yCounter = 0;//startPos;
    let l = 0;
    for(const {elem, bbox} of elemBboxes){
      const x = bbox.width*((anchor & HorizontalAnchor.Right)?0:((anchor&HorizontalAnchor.Center)?0:0));
      elem.setAttributeNS(null, "x", x+"");
      elem.setAttributeNS(null, "y", (yCounter)+"");
      //console.log(x,yCounter, elem);
      yCounter+=bbox.height*((l===0||l===(elemBboxes.length-1))?1:multiplier);
      l++
    }
    g.setAttributeNS(null, 'transform', `translate(${position.x}, ${position.y-heightSum})`)
    if(fitToContainer){
      const bbox = g.getBBox();
      const scale = Math.min(fitToContainer.width/bbox.width, fitToContainer.height/bbox.height);
      if(scale<0.9){
      g.setAttributeNS(null, 'transform', `translate(${position.x}, ${position.y-heightSum}) scale(${scale})`)
      }
    }
  };
  return {
    g,
    setText,
  }
}
function domRectToSize(rect: DOMRect): ISize{
  return {
    width: rect.width,
    height: rect.height,
  }
}
type TextLineElementWithBB = {line: SVGGraphicsElement, size: ISize};
function createSizedCentredTextLines(measureCanvas: SVGGElement | SVGSVGElement, text: string, anchor: TNodeAnchor, props: any = {}): TextLineElementWithBB[]{
  const lines = text.split("\n").map(l=>decomposeLine(l));
  const elemBBs: TextLineElementWithBB[] = [];
  for(let l=0;l<lines.length;l++){
    const line = lines[l];
    const elem = createGraphicsElement("text", {x:0, y: 0, ...props});
    elem.style.textAnchor = "middle";
    elem.style.dominantBaseline = (anchor&VerticalAnchor.Top)?"text-before-edge":((anchor&VerticalAnchor.Center)?"middle":"text-after-edge");
    for(let tspan of line){
      elem.appendChild(tspan);
    }
    measureCanvas.appendChild(elem);
    const bbox = elem.getBBox();
    measureCanvas.removeChild(elem);
    //console.log("got bbox",bbox)
    elemBBs.push({line: elem, size: domRectToSize(bbox)});

  }
  return elemBBs;
}
function alignTextCenteredLines2(parent: SVGGElement, lines: TextLineElementWithBB[],point: IVec2, anchor: TNodeAnchor,lineHeightMultiplier: number = 1.15, fitToContainer?: ISize){
  let totalWidth = 0;
  let totalHeight = 0;
  let lastOffset = 0;
  const yValues : number[] = [];
  const xValues : number[] = [];
  for(let l=0;l<lines.length;l++){
    const line = lines[l];
    yValues.push(totalHeight);
    totalHeight+=line.size.height*lineHeightMultiplier;
    totalWidth = Math.max(totalWidth, line.size.width);
  }

  
  for(let l=0;l<lines.length;l++){
    const line = lines[l];
    xValues.push((line.size.width/2));
  }
  const yMul = (anchor&VerticalAnchor.Bottom)?1:(anchor&VerticalAnchor.Center?0.5:0)/lineHeightMultiplier;
  const container = createGraphicsElement("g", {"transform": `translate(-${totalWidth/2}, -${(totalHeight-lines[0].size.height)*yMul})`});
  lines.forEach((x,i)=>{
    x.line.setAttributeNS(null, "x", ((totalWidth-lines[i].size.width)/2)+xValues[i]+"");
    x.line.setAttributeNS(null, "y", (yValues[i])+"");
    container.appendChild(x.line);
  });

  
  if(fitToContainer){
    parent.appendChild(container);
    const bbox = container.getBBox();
    parent.removeChild(container);
    const scale = Math.min(fitToContainer.width/bbox.width, fitToContainer.height/bbox.height);
    //console.log("ftc",scale);
    if(scale<0.9){
      container.setAttributeNS(null, 'transform', `translate(-${totalWidth/2*scale}, -${(totalHeight-lines[0].size.height)*yMul*scale}) scale(${scale})`)
    }
  }
  return container;

  

}
function alignTextCenteredLines(lines: TextLineElementWithBB[],point: IVec2, anchor: TNodeAnchor,lineHeightMultiplier: number = 1.15, fitToContainer?: ISize){
  let totalWidth = 0;
  let totalHeight = 0;
  let lastOffset = 0;
  const yValues : number[] = [];
  for(let l=0;l<lines.length;l++){
    const line = lines[l];
    totalWidth = Math.max(totalWidth, line.size.width);
    const halfMupltiplierOffset = line.size.height*(1-lineHeightMultiplier)/2;
    const yStart = totalHeight+line.size.height/2+lastOffset+l===0?0:halfMupltiplierOffset;
    totalHeight+=yStart-totalHeight+line.size.height/2;
    lastOffset = halfMupltiplierOffset;
    yValues.push(yStart);
  }
  const yValuesOffset = -totalHeight*((anchor&VerticalAnchor.Bottom)?1:((anchor&VerticalAnchor.Center)?0.5:0));
  

  const xValueOffsets = lines.map(x=>{
    const width = x.size.width;
    if(anchor&HorizontalAnchor.Right){
      return -width/2;
    }else if(anchor&HorizontalAnchor.Center){
      return 0;
    }else{
      return width/2;
    }
  });
  const container = createGraphicsElement("g", {"transform": `translate(${point.x}, ${point.y})`});
  lines.forEach((x,i)=>{
    x.line.setAttributeNS(null, "x", xValueOffsets[i]+"");
    x.line.setAttributeNS(null, "y", (yValues[i]+yValuesOffset)+"");
    container.appendChild(x.line);
  });
  const outerContainer = createGraphicsElement("g");
  outerContainer.appendChild(container);

  return outerContainer;
}
function multilineTextGroupV3(measureCanvas: SVGGElement | SVGSVGElement, position: IVec2, props: any = {}, multiplier=1.15, anchor = NodeAnchor.Center, bgGen?: (size: ISize)=>SVGGraphicsElement, fitToContainer?: ISize){
  const outerCon = createGraphicsElement("g", {transform: `translate(${position.x},${position.y})`});
  //console.log("position",position);

  const setText = (str: string)=>{
    while(outerCon.lastChild){
      outerCon.removeChild(outerCon.lastChild);
    }
    outerCon.style.transform ="none";
    outerCon.style.transform ="";
    const lines = createSizedCentredTextLines(measureCanvas, str, anchor, props);
    //console.log("lines",lines)
    const container = alignTextCenteredLines2(outerCon, lines, {x: 0, y: 0}, anchor, multiplier, fitToContainer);
    
    if(bgGen){
      measureCanvas.appendChild(container);
      const size=domRectToSize(container.getBBox());
      measureCanvas.removeChild(container);
      const bg = bgGen(size);
      outerCon.appendChild(bg);
    }
    outerCon.appendChild(container);
  };

  return {
    g: outerCon,
    setText,
  } 

}
export type {
  ITextElementHelper,
}
export {
  createGroupElement,
  createGraphicsElement,
  createPathElement,
  multilineTextGroup,
  multilineTextGroupV2,
  multilineTextGroupV3,
  svgRichText,
  domRectToSize,

}