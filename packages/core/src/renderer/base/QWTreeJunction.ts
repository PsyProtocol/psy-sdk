import { simpleStateDiff } from "../../diff";
import { layoutDirectional, layoutHorizontal } from "../../layout/directional";
import { IBoundingBox, ISize, IVec2 } from "../../types/geo";
import { ITreeJunctionLayout } from "../../types/layout";
import { IQSRenderContext, IQSWidgetDeserializer } from "../../types/renderer";
import { IQWidgetSerialized, ISimpleEdgeDef } from "../../types/scene";
import { QWidget, QWidgetWithSimpleEdges } from "./QWidget";
import { makeSVGElemAttributes } from "@qstudio/qsvg";

interface IQWTreeJunctionConfig {
  layout: ITreeJunctionLayout;
}
interface IQWTreeJunctionState {
}

type IQWTreeJunctionStatePatch = Partial<IQWTreeJunctionState>;

class QWTreeJunction extends QWidgetWithSimpleEdges<IQWTreeJunctionConfig, IQWTreeJunctionState, IQWTreeJunctionStatePatch> {
  rootNode: QWidget<any, any, any> = null as any;
  childrenNodes: QWidget<any, any, any>[] = [];

  static create(root: QWidget<any, any, any>, children: QWidget<any, any, any>[], config: IQWTreeJunctionConfig): QWTreeJunction {
    const w = new QWTreeJunction(config);
    w.childrenNodes = children;
    w.rootNode = root;
    return w;
  }
  static deserialize(deserializer: IQSWidgetDeserializer, serialized: IQWidgetSerialized<any, any>): QWTreeJunction {
    const allChildren = serialized.children?.map(x => deserializer.deserializeWidget<any, any, any>(x)) ?? [];
    if(allChildren.length === 0){
      throw new Error("QWTreeJunction must have at least one child");
    }


    let [rootNode, ...childrenNodes] = allChildren;

    return QWTreeJunction.create(rootNode, childrenNodes, serialized.config);
  }

  getChildren(): QWidget<any, any, any>[] {
    return [this.rootNode, ...this.childrenNodes];
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    this.size.height =  this.config.layout.levelSpacing+this.rootNode.size.height + this.childrenNodes[0].size.height;

    const res = layoutDirectional(this.rootNode.size, this.childrenNodes.map(x => x.size), this.config.layout);
    this.rootNode.position = {x: 0, y: -this.size.height/2 +this.rootNode.size.height/2};
    for (let i = 0; i < this.childrenNodes.length; i++) {
      this.childrenNodes[i].position.x = res.children[i].center.x;
      this.childrenNodes[i].position.y = this.config.layout.levelSpacing + this.rootNode.size.height/2 + this.childrenNodes[i].size.height/2;
      //this.childrenNodes[i].position.y = this.rootNode.position.y + this.config.layout.levelSpacing + this.rootNode.size.height + this.childrenNodes[i].size.height/2;
    }
    this.size = res.container.size;
    this.size.height =  this.config.layout.levelSpacing+this.rootNode.size.height + this.childrenNodes[0].size.height;
    return {
      center: this.position,
      size: this.size,
    };
  }
  getWidgetType(): string {
    return "QWTreeJunction";
  }

  labelElement?: SVGTextElement;
  rectElement?: SVGRectElement;

  getDefaultState(): IQWTreeJunctionState {
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
      className: this.config.layout.edgeClassName,
    }));
  }
  renderInternalBase(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");

    return g;
  }
  applyStateUpdate(stateUpdate: Partial<IQWTreeJunctionState>): boolean {
    const { newState, diff, changed } = simpleStateDiff(this.state, stateUpdate);
    if (!changed) {
      return false;
    }
    return false;
  }
}


export {
  QWTreeJunction,
}

export type {
  IQWTreeJunctionConfig,
}