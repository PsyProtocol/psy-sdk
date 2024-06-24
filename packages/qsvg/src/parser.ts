const RAW_TEXT_TAG_NAME = 'RAW_TEXT';

function getCounter() {
  let counter = 0;
  return () => {
    return counter++;
  };
}

function uniqueIdGenerator(){
  let counter = 0;
  return () => {
    return (counter++).toString();
  };
}
interface IParsedSVGNode {
  uniqueId: string;
  tagName: string;
  attributes: Record<string, string>;
  children: IParsedSVGNode[];
}
interface IFlatSVGNode {
  uniqueId: string;
  tagName: string;
  attributes: Record<string, string>;
  children: string[];
}

function walkParsedSVGNode(node: IParsedSVGNode, callback: (node: IParsedSVGNode)=>void){
  callback(node);
  node.children.forEach(c=>walkParsedSVGNode(c, callback));
}
function generateParsedSVGNode(node: Node, idGenerator: ()=>string, ignoreEmptyWhitespace: boolean = false): IParsedSVGNode | null {
  if(node.nodeType === Node.TEXT_NODE){
    if(ignoreEmptyWhitespace && node.textContent?.trim() === ''){
      return null;
    }
    return {
      uniqueId: idGenerator(),
      tagName: RAW_TEXT_TAG_NAME,
      attributes: {"textContent": node.textContent ?? ''},
      children: [],
    };
  }else if(node.nodeType === Node.ELEMENT_NODE){
    const id = idGenerator();
    const elem = node as Element;
    const attributes: Record<string, string> = {};
    elem.getAttributeNames().filter(x=>x!=="xmlns"&&x.substring(0,4)!=="xml:").forEach(x=>attributes[x]= (elem.getAttribute(x) ?? ''));
    const tagName = elem.nodeName;
    const hasElementChildren = elem.childElementCount > 0;
    const children = Array.from(elem.childNodes).map(c=>generateParsedSVGNode(c, idGenerator, hasElementChildren)).filter(x=>x !== null) as IParsedSVGNode[];
    return {
      uniqueId:id,
      tagName,
      attributes,
      children,
    };
  }else{
    return null;
  }
}
function parseSVG(svgRaw: string): IParsedSVGNode {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svgRaw, 'image/svg+xml');
  const parsed = generateParsedSVGNode(doc.children[0], uniqueIdGenerator());
  if(!parsed){
    throw new Error("Failed to parse SVG");
  }else{
    return parsed;
  }
}

function generateElemDefCode(elem: IFlatSVGNode): string {
  if(elem.tagName === RAW_TEXT_TAG_NAME){
    return `const el_${elem.uniqueId} = document.createTextNode(${JSON.stringify(elem.attributes.textContent)});`;
  }
  const prefix = "el_";
  const def = `makeSVGElement("${elem.tagName}", {}, ${JSON.stringify(elem.attributes)})`;
  const line = `const ${prefix}${elem.uniqueId} = ${def};`;
  return line;
}
function genAppendCode(elem: IFlatSVGNode): string {
  const prefix = "el_";
  const parent = elem.uniqueId;
  const children = elem.children.map(x=>prefix+x).join(", ");
  return `appendChildren(${prefix+parent}, [${children}]);`;
}
function genJSCodeForSVGNode(node: IParsedSVGNode): string {
  const elements: IFlatSVGNode[] = [];
  walkParsedSVGNode(node, (n)=>{
    elements.push({
      uniqueId: n.uniqueId,
      tagName: n.tagName,
      attributes: n.attributes,
      children: n.children.map(x=>x.uniqueId),
    });
  });

  const defCode = elements.map(generateElemDefCode).join("\n");
  const appendCode = elements.filter(x=>x.children.length).map(genAppendCode).join("\n");
  return `${defCode}\n${appendCode}`;
}
function genCodeForSVG(svgRaw: string){
  const parsed = parseSVG(svgRaw);
  return genJSCodeForSVGNode(parsed);
}
export {
  parseSVG,
  genJSCodeForSVGNode,
  genCodeForSVG,
}