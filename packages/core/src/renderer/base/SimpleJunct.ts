import { NodeAnchor, VerticalAnchor } from "../../anchor";
import { simpleStateDiff } from "../../diff";
import { layoutDirectional, layoutHorizontal } from "../../layout/directional";
import { IBoundingBox, ISize, IVec2 } from "../../types/geo";
import { ITreeJunctionLayout } from "../../types/layout";
import { IQSRenderContext, IQSWidgetDeserializer } from "../../types/renderer";
import { IQWidgetSerialized, ISimpleEdgeDef } from "../../types/scene";
import { QWidget, QWidgetWithSimpleEdges } from "./QWidget";
import { makeSVGElemAttributes } from "@qstudio/qsvg";

interface IQWSimpleJunctionConfig {
  layout: ITreeJunctionLayout;
}
interface IQWSimpleJunctionState {
}

type IQWSimpleJunctionStatePatch = Partial<IQWSimpleJunctionState>;

class QWSimpleJunction extends QWidgetWithSimpleEdges<IQWSimpleJunctionConfig, IQWSimpleJunctionState, IQWSimpleJunctionStatePatch> {
  rootNode: QWidget<any, any, any> = null as any;
  childrenNodes: QWidget<any, any, any>[] = [];

  static create(root: QWidget<any, any, any>, children: QWidget<any, any, any>[], config: IQWSimpleJunctionConfig): QWSimpleJunction {
    const w = new QWSimpleJunction(config);
    w.childrenNodes = children;
    w.rootNode = root;
    return w;
  }
  static deserialize(deserializer: IQSWidgetDeserializer, serialized: IQWidgetSerialized<any, any>): QWSimpleJunction {
    const allChildren = serialized.children?.map(x => deserializer.deserializeWidget<any, any, any>(x)) ?? [];
    if(allChildren.length === 0){
      throw new Error("QWSimpleJunction must have at least one child");
    }


    let [rootNode, ...childrenNodes] = allChildren;

    return QWSimpleJunction.create(rootNode, childrenNodes, serialized.config);
  }

  getChildren(): QWidget<any, any, any>[] {
    return [this.rootNode, ...this.childrenNodes];
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    const res = layoutHorizontal(this.childrenNodes.map(x => x.size), this.config.layout.siblingSpacing, NodeAnchor.TopCenter);
    
    let averageX = res.children.map(x=>x.center.x).reduce((a,b)=>a+b, 0) / res.children.length;

    this.rootNode.position = {x: averageX, y: this.position.y};

    for (let i = 0; i < this.childrenNodes.length; i++) {
      this.childrenNodes[i].position.x = res.children[i].center.x-averageX;
      this.childrenNodes[i].position.y = this.position.y + this.config.layout.levelSpacing+res.container.size.height/2;
    }
    this.size = res.container.size;
    return {
      center: this.position,
      size: this.size,
    };
  }
  getWidgetType(): string {
    return "QWSimpleJunction";
  }

  labelElement?: SVGTextElement;
  rectElement?: SVGRectElement;

  getDefaultState(): IQWSimpleJunctionState {
    return {
      label: "",
      textColor: "#111",
      color: "#f2f2f2",
      borderColor: "#444",
    };
  }
  getEdges(): ISimpleEdgeDef[] {
    if(!(this.rootNode as any)){
      return [];
    }
    return this.childrenNodes.map(x=>({
      fromRef: {id: this.rootNode.id, anchor: this.config.layout.parentAnchor},
      toRef: {id: x.id, anchor: this.config.layout.childAnchor},
    }));
  }
  renderInternalBase(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");

    const rect = makeSVGElemAttributes("rect", {
      fill: "#f0f",
      width: "8px",
      height: "8px",
      "stroke-width": 1 + "px",
      x: -4,
      y: -4,
    });
    rect.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      console.log("clicked");
      this.rootNode.updateState({ color: "#0ff" });
    });
    g.appendChild(rect);
    return g;
  }
  applyStateUpdate(stateUpdate: Partial<IQWSimpleJunctionState>): boolean {
    const { newState, diff, changed } = simpleStateDiff(this.state, stateUpdate);
    if (!changed) {
      return false;
    }
    return false;
  }
}


export {
  QWSimpleJunction,
}