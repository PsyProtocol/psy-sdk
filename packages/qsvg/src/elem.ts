function createTagWithRefs(tagNameRefs: string, ns?: string) {
  const parts = tagNameRefs.split(/(?=[.#])/);
  const tagName = parts.shift()!;
  const element = ns ? document.createElementNS(ns, tagName) : document.createElement(tagName);

  parts.forEach(part => {
    if (part.startsWith(".")) {
      element.classList.add(part.slice(1));
    } else if (part.startsWith("#")) {
      element.id = part.slice(1);
    }
  });

  return element;
}

function makeElementFull(tagNameRefs: string, ns?: string, props: any = {}, attributes: any = {}, children: any[] = []) {
  const element = createTagWithRefs(tagNameRefs, ns);
  Object.keys(attributes).forEach(key => {
    if(key == 'xmlns') {
      return;
    }
    if (key === 'textContent') {
      element.textContent = attributes[key];
    } else {
      if (typeof attributes[key] === 'undefined') {
        //element.removeAttribute(key);
      } else {
        element.setAttributeNS(null, key, attributes[key]);
      }
    }
  });

  if (ns === "http://www.w3.org/2000/svg") {
    Object.keys(props).forEach(key => {
      if (key === 'className') {
        element.setAttributeNS(null, "class", props[key]);
      } else {
        (element as any)[key] = props[key];
      }
    });
  } else {
    Object.keys(props).forEach(key => {
      (element as any)[key] = props[key];
    });
  }

  children.forEach(child => {
    element.appendChild(child);
  });
  return element;


}
function makeElemAttributes(tagNameRefs: string, propsOrChildren?: any, childrenOrUndefined?: any) {
  const attributes = Array.isArray(propsOrChildren) ? {} : propsOrChildren;
  const children = Array.isArray(propsOrChildren) ? propsOrChildren : (childrenOrUndefined || []);
  return makeElementFull(tagNameRefs, "", {}, attributes, children);
}

function makeSVGElemAttributes<T = Element>(tagNameRefs: string, propsOrChildren?: any, childrenOrUndefined?: any): T {
  const attributes = Array.isArray(propsOrChildren) ? {} : propsOrChildren;
  const children = Array.isArray(propsOrChildren) ? propsOrChildren : (childrenOrUndefined || []);
  return makeElementFull(tagNameRefs, "http://www.w3.org/2000/svg", {}, attributes, children) as T;
}

function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, children: Element[]): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, attributes: Record<string, any>): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, children: Element[]): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, attributes: Record<string, any>, children: Element[]): SVGElementTagNameMap[K];
function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, propsOrChildren?: any, attributesOrChildren?: any, children?: any): SVGElementTagNameMap[K] {

  if (typeof propsOrChildren === 'undefined') {
    // 1 argument
    // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K): SVGElementTagNameMap[K];
    return makeElementFull(tagName, "http://www.w3.org/2000/svg", {}, {}, []) as SVGElementTagNameMap[K];
  } else if (typeof attributesOrChildren === 'undefined') {
    // 2 arguments
    if (Array.isArray(propsOrChildren)) {
      // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, children: Element[]): SVGElementTagNameMap[K];
      return makeElementFull(tagName, "http://www.w3.org/2000/svg", {}, {}, propsOrChildren) as SVGElementTagNameMap[K];
    } else {
      // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>): SVGElementTagNameMap[K];
      return makeElementFull(tagName, "http://www.w3.org/2000/svg", propsOrChildren, {}, []) as SVGElementTagNameMap[K];
    }
  } else if (typeof children === 'undefined') {
    // 3 arguments
    if (Array.isArray(attributesOrChildren)) {
      // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, children: Element[]): SVGElementTagNameMap[K];
      return makeElementFull(tagName, "http://www.w3.org/2000/svg", propsOrChildren, {}, attributesOrChildren) as SVGElementTagNameMap[K];
    } else {
      // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, attributes: Record<string, any>): SVGElementTagNameMap[K];
      return makeElementFull(tagName, "http://www.w3.org/2000/svg", propsOrChildren, attributesOrChildren, []) as SVGElementTagNameMap[K];
    }
  } else {
    // 4 arguments
    // function makeSVGElement<K extends keyof SVGElementTagNameMap>(tagName: K, props: Partial<SVGElementTagNameMap[K]>, attributes: Record<string, any>, children: Element[]): SVGElementTagNameMap[K];
    return makeElementFull(tagName, "http://www.w3.org/2000/svg", propsOrChildren, attributesOrChildren, children) as SVGElementTagNameMap[K];
  }
}

function createGraphicsElement(tagName: string, props: any = {}): SVGGraphicsElement {
  const elem = document.createElementNS("http://www.w3.org/2000/svg", tagName);
  Object.keys(props).forEach(key => {
    if (key === 'textContent') {
      elem.textContent = props[key];
    } else {
      elem.setAttributeNS(null, key, props[key]);
    }
  });
  return elem as SVGGraphicsElement;
}
function createPathElement(startPoint: { x: number, y: number }, endPoint: { x: number, y: number }, props: any = {}): SVGPathElement {
  const path = createGraphicsElement("path", props);
  path.setAttributeNS(null, "d", `M ${startPoint.x} ${startPoint.y} L ${endPoint.x} ${endPoint.y}`);
  return path as SVGPathElement;
}

export {
  makeElemAttributes,
  makeSVGElemAttributes,
  makeSVGElement,
  makeElementFull,
  createTagWithRefs,
  createPathElement,
}