import { IAnchorRef, IBoundingBox, ISize, IVec2 } from "../../types/geo";
import {
  IQSRenderContext,
  IQSWidgetDeserializer,
  IQStudioSceneManager,
} from "../../types/renderer";
import {
  IQWidgetSerialized,
  ISimpleEdgeDef,
  IResolvedSimpleEdgeDefinition,
} from "../../types/scene";
import { bbox, v2m } from "../../vecmath";
import { v4 as uuidv4 } from "uuid";
import { renderSimpleEdge } from "../edge";
import { createGraphicsElement } from "../text/helpers";
import { makeSVGElement } from "@qstudio/qsvg";

function getElementAbsoluteCenter(svg: SVGSVGElement, element: any): IVec2 {
  const bbox = element.getBBox();
  const pt = svg.createSVGPoint();
  pt.x = bbox.x + bbox.width / 2;
  pt.y = bbox.y + bbox.height / 2;
  return pt.matrixTransform(element.getCTM()!);
}
abstract class QWidget<C, S, U> {
  id: string;
  state: S;
  config: C;
  lastRenderedContainer: SVGGElement | null = null;
  lastRendered: SVGGElement | null = null;
  position: IVec2;
  size: ISize;

  constructor(
    config: C,
    position?: IVec2,
    size?: ISize,
    initialState?: S,
    id?: string
  ) {
    this.config = config;
    this.state =
      typeof initialState === "undefined"
        ? this.getDefaultState()
        : initialState;
    this.position = position || { x: 0, y: 0 };
    this.size = size || { width: -1, height: -1 };
    this.id = id || uuidv4();
  }

  serialize(): IQWidgetSerialized<C, S> {
    return {
      id: this.id,
      type: this.getWidgetType(),
      config: this.config,
      state: this.state,
      position: this.position,
      size: this.size,
      children: this.getChildren().map((child) => child.serialize()),
    };
  }

  static deserialize(
    deserializer: IQSWidgetDeserializer,
    serialized: IQWidgetSerialized<any, any>
  ): QWidget<any, any, any> {
    throw new Error("deserialize not implemented on the base class");
  }

  abstract getChildren(): QWidget<any, any, any>[];
  abstract getWidgetType(): string;

  abstract getDefaultState(): S;

  abstract renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement;
  abstract applyStateUpdate(stateUpdate: U): boolean;

  render(context: IQSRenderContext, parent: SVGGElement) {
    if (this.lastRenderedContainer) {
      if (this.lastRenderedContainer.parentElement) {
        this.lastRenderedContainer.parentElement.removeChild(
          this.lastRenderedContainer
        );
      }
    }
    const container = document.createElementNS(
      "http://www.w3.org/2000/svg",
      "g"
    );
    const inner = this.renderInternal(context, container);
    const children = this.getChildren();
    if (children.length > 0) {
      const childrenContainer = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "g"
      );
      inner.appendChild(childrenContainer);
      children.forEach((child) => child.render(context, childrenContainer));
    }

    container.appendChild(inner);

    this.lastRenderedContainer = container;
    this.lastRendered = inner;

    context.measurePaper.appendChild(container);
    const bboxNew = bbox(container.getBBox());
    context.measurePaper.removeChild(container);
    this.size = bboxNew.size;

    parent.appendChild(container);

    container.setAttribute(
      "transform",
      `translate(${this.position.x}, ${this.position.y})`
    );
    container.dataset.qWidgetType = this.getWidgetType();
    container.dataset.qWidgetId = this.id;
    container.dataset.qWidgetConfig = JSON.stringify(this.config);
    //container.setAttribute('transform', `translate(${-bboxNew.size.width/2},${-bboxNew.size.height/2})`);
    //`transform-origin: ${this.position.x}px ${this.position.y}px;
    //inner.setAttributeNS(null, "transform", `translate(${this.position.x-bboxNew.size.width/2},${this.position.y-bboxNew.size.height/2})`);
    /*container.setAttributeNS(
      null,
      "style",`transform: translate(${
        this.position.x - bboxNew.size.width / 2
      }px,${this.position.y - bboxNew.size.height / 2}px)`
    );*/
    return container;
  }

  abstract layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox;

  layout(): IBoundingBox {
    const childBBoxes = this.getChildren().map((child) => child.layout());
    const bbox = this.layoutInternal(childBBoxes);
    this.position = bbox.center;
    this.size = bbox.size;
    return bbox;
  }

  getBBox(): IBoundingBox {
    return {
      center: this.position,
      size: this.size,
    };
  }
  getClonedElement(): SVGGElement {
    if (!this.lastRenderedContainer) {
      throw new Error("getClonedElement: lastRenderedContainer is null");
    }
    const node = this.lastRenderedContainer.cloneNode(true) as SVGGElement;
    return node;
  }
  getAbsoluteCenter(context: IQSRenderContext): IVec2 {
    if (!this.lastRenderedContainer) {
      throw new Error("getAbsoluteCenter: lastRenderedContainer is null");
    }
    return getElementAbsoluteCenter(context.svg, this.lastRenderedContainer);
  }

  updateState(update: U): boolean {
    return this.applyStateUpdate(update);
  }

  consume(
    context: IQSRenderContext,
    widget: QWidget<any, any, any>,
    duration = 1000
  ) {
    const destCenter = this.getAbsoluteCenter(context);
    const sourceCenter = widget.getAbsoluteCenter(context);
    const transformed100 = context.svg.createSVGPoint();
    transformed100.x = 100;
    transformed100.y = 100;
    const transformed0 = context.svg.createSVGPoint();
    transformed0.x = 0;
    transformed0.y = 0;

    const testDotWrapperOuter = createGraphicsElement("g");

    const testDotWrapper = createGraphicsElement("g");
    testDotWrapperOuter.appendChild(testDotWrapper);

    context.paper.appendChild(testDotWrapperOuter);

    const transformed0b = transformed0.matrixTransform(
      testDotWrapper.getCTM()!
    );
    const transformed100b = transformed100.matrixTransform(
      testDotWrapper.getCTM()!
    );

    const dif = {
      x: transformed100b.x - transformed0b.x,
      y: transformed100b.y - transformed0b.y,
    };
    const ratio = 100 / dif.x;

    /*
    const baseCenter = getElementAbsoluteCenter(context.svg, testDotWrapper);


    const offset2 = {x: destCenter.x-baseCenter.x, y: destCenter.y-baseCenter.y};
    const offset3 = {x: sourceCenter.x-baseCenter.x, y: sourceCenter.y-baseCenter.y};

    const destOffset = v2m.mulScalar(offset2, ratio);
    const srcOffset = v2m.mulScalar(offset3, ratio);
    */
    const cloned = widget.getClonedElement();
    testDotWrapper.appendChild(cloned);

    const clonedOffset = getElementAbsoluteCenter(context.svg, cloned);

    const offset5 = {
      x: sourceCenter.x - clonedOffset.x,
      y: sourceCenter.y - clonedOffset.y,
    };

    const nwOffset = v2m.mulScalar(offset5, ratio);
    const offset6 = {
      x: destCenter.x - clonedOffset.x,
      y: destCenter.y - clonedOffset.y,
    };

    const nwOffsetb = v2m.mulScalar(offset6, ratio);

    testDotWrapperOuter.setAttribute(
      "transform",
      `translate(${nwOffset.x}, ${nwOffset.y})`
    );

    let innerBBox = testDotWrapper.getBBox();
    testDotWrapper.style.transformOrigin = `${
      innerBBox.x + innerBBox.width / 2
    }px ${innerBBox.y + innerBBox.height / 2}px`;
    setTimeout(() => {
      const timing = `cubic-bezier(1.000, 0.290, 0.390, 0.720)`;
      testDotWrapper.style.transition = `transform ${duration}ms ${timing}, opacity ${duration}ms ${timing}`;
      testDotWrapperOuter.style.transition = `transform ${duration}ms ease-out, opacity ${duration}ms cubic-bezier(0.605, 0.545, 1.000, -0.280)`;

      testDotWrapperOuter.setAttribute(
        "transform",
        `translate(${nwOffsetb.x}, ${nwOffsetb.y})`
      );
      testDotWrapper.setAttribute("transform", `scale(${0.1})`);

      setTimeout(() => {
        testDotWrapperOuter.style.opacity = "0";
        setTimeout(() => {
          context.paper.removeChild(testDotWrapperOuter);
        }, 100);
      }, Math.max(duration/6, duration-50));
    }, 10);

    //testDotWrapper.setAttribute('transform', `translate(${nwOffsetb.x}, ${nwOffsetb.y})`);

    /*
   const wrapper  = createGraphicsElement("g");


    const cloned = widget.getClonedElementAbsolute(context.paper);
    wrapper.appendChild(cloned);
    context.paper.appendChild(wrapper);
    const destBBox = this.lastRenderedContainer!.getBBox();
    console.log("destBBox",destBBox);
    const sourceBBox = cloned.getBBox();
    console.log("sourceBBox",sourceBBox);
    const offset = {x: destBBox.x-sourceBBox.x, y: destBBox.y-sourceBBox.y};
    wrapper.setAttribute('transform', `translate(${offset.x}, ${offset.y})`);*/

    /*
    const testG = createGraphicsElement("g");
    testG.style.opacity = '0';
    const testG2 = createGraphicsElement("g");
    
    paper.appendChild(testG2);
    testG.appendChild(widget.getClonedElementAbsolute());
    testG2.appendChild(testG);
    const otherCloneBounding = testG.getBoundingClientRect();
    const otherRealBounding = widget.lastRenderedContainer!.getBoundingClientRect();
    const cloneOffset = {x: otherRealBounding.x - otherCloneBounding.x, y: otherRealBounding.y - otherCloneBounding.y};
    testG2.setAttribute('transform', `translate(${cloneOffset.x}, ${cloneOffset.y})`);
    testG.style.opacity = '1';
    const bboxG = testG.getBBox();
    const ourBBox = this.lastRenderedContainer!.getBBox();
    const centerG = {x: bboxG.x+bboxG.width/2, y: bboxG.y+bboxG.height/2};
    const centerOur = {x: ourBBox.x+ourBBox.width/2, y: ourBBox.y+ourBBox.height/2};
    const dif = {x: centerOur.x-centerG.x, y: centerOur.y-centerG.y};
    testG.style.transformOrigin = `${centerG.x}px ${centerG.y}px`;
    testG2.style.transformOrigin = `${centerG.x}px ${centerG.y}px`;
    const timing = `cubic-bezier(1.000, 0.290, 0.390, 0.720)`
    testG.style.transition=`transform ${duration}ms ${timing}, opacity ${duration}ms ${timing}`;
    testG2.style.transition=`transform ${duration}ms ease-out, opacity ${duration}ms cubic-bezier(0.605, 0.545, 1.000, -0.280)`;
    
    testG.style.transform=`scale(${Math.min(ourBBox.width/bboxG.width, ourBBox.height/bboxG.height)/2})`;
    testG2.style.transform = `translate(${dif.x}px,${dif.y}px)`;
    setTimeout(()=>{
      //testG2.style.opacity = '0';
      setTimeout(()=>{
        //paper.removeChild(testG2);
      }, duration+100);
    },duration);
    return testG2;
    */
  }
}

abstract class QWidgetWithEdges<C, S, U> extends QWidget<C, S, U> {
  abstract renderEdges(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement | null;
  abstract renderInternalBase(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement;
  renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const internalBase = this.renderInternalBase(context, container);
    const edges = this.renderEdges(context, container);
    if (edges) {
      const edgeContainer = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "g"
      );
      edgeContainer.appendChild(edges);
      edgeContainer.appendChild(internalBase);
      return edgeContainer;
    } else {
      return internalBase;
    }
  }
}
function resolveEdgeDefRef(
  context: IQSRenderContext,
  point: IVec2 | undefined,
  ref: IAnchorRef | undefined
): IVec2 {
  if (typeof point !== "undefined") {
    return point;
  } else if (typeof ref !== "undefined") {
    const result = context.manager.getWidgetAnchorPoint(
      ref.id,
      ref.anchor,
      ref.offset
    );
    if (!result) {
      return { x: 0, y: 0 };
    } else {
      return result;
    }
  } else {
    throw new Error("resolveEdgeDefRef: point and ref are both undefined");
  }
}
function resolveEdgeDef(
  context: IQSRenderContext,
  edgeDef: ISimpleEdgeDef
): IResolvedSimpleEdgeDefinition {
  const from = resolveEdgeDefRef(context, edgeDef.from, edgeDef.fromRef);
  const to = resolveEdgeDefRef(context, edgeDef.to, edgeDef.toRef);
  return {
    ...edgeDef,
    from,
    to,
  };
}
abstract class QWidgetWithSimpleEdges<C, S, U> extends QWidgetWithEdges<
  C,
  S,
  U
> {
  renderedEdges: (SVGPathElement | SVGGElement)[] = [];
  edgeElementMap: Record<string, SVGPathElement | SVGGElement> = {};

  abstract getEdges(): ISimpleEdgeDef[];
  renderEdges(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement | null {
    this.edgeElementMap = {};
    this.renderedEdges = [];
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    const edgeDefs = this.getEdges().map((x) => resolveEdgeDef(context, x));
    const edgeElements = edgeDefs.map((x, i) => {
      const id = x.id || `${this.id}_no-id_edge_${i}`;
      const elem = renderSimpleEdge(x);
      g.appendChild(elem);
      this.edgeElementMap[id] = elem;
      return elem;
    });
    this.renderedEdges = edgeElements;
    return g;
  }
}
export { QWidget, QWidgetWithEdges, QWidgetWithSimpleEdges };
