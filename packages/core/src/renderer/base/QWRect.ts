import { simpleStateDiff } from "../../diff";
import { IBoundingBox } from "../../types/geo";
import { IQSRenderContext } from "../../types/renderer";
import { QWidget } from "./QWidget";
import { makeSVGElemAttributes } from "@qstudio/qsvg";

interface IQWRectConfig {
  width: number;
  height: number;
  borderWidth: number;
}
interface IQWRectState {
  label: string;
  color: string;
  textColor: string;
  borderColor: string;
}

type IQWRectStatePatch = Partial<IQWRectState>;

class QWRect extends QWidget<IQWRectConfig, IQWRectState, IQWRectStatePatch> {
  getChildren(): QWidget<any, any, any>[] {
    return [];
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    return {
      center: this.position,
      size: {
        width: this.config.width,
        height: this.config.height,
      },
    };
  }
  getWidgetType(): string {
    return "QWRect";
  }

  labelElement?: SVGTextElement;
  rectElement?: SVGRectElement;
  
  getDefaultState(): IQWRectState {
    return {
      label: "",
      textColor: "#111",
      color: "#f2f2f2",
      borderColor: "#444",
    };
  }
  renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    const rect = makeSVGElemAttributes("rect", {
      fill: this.state.color,
      width: this.config.width + "px",
      height: this.config.height + "px",
      "stroke-width": this.config.borderWidth + "px",
      x: -this.config.width/2,
      y: -this.config.height/2,
    });
    const label = makeSVGElemAttributes("text", {
      x: 0,
      y: 0,
      "text-anchor": "middle",
      "dominant-baseline": "middle",
      fill: "black",
      font: "12px sans-serif",
      textContent: this.state.label,
    });
    this.rectElement = rect as SVGRectElement;
    this.labelElement = label as SVGTextElement;

    g.appendChild(rect);
    g.appendChild(label)
    return g;
  }
  applyStateUpdate(stateUpdate: Partial<IQWRectState>): boolean {
    const {newState, diff, changed} = simpleStateDiff(this.state, stateUpdate);
    if(!changed){
      return false;
    }
    this.state = newState;
    if(diff.label && this.labelElement){
      this.labelElement.textContent = diff.label;
    }else if(diff.color && this.rectElement){
      this.rectElement.setAttribute("fill", diff.color);
    }else if(diff.borderColor && this.rectElement){
      this.rectElement.setAttribute("stroke", diff.borderColor);
    }else if(diff.textColor && this.labelElement){
      this.labelElement.setAttribute("fill", diff.textColor);
    }
    return false;
  }
}


export {
  QWRect,
}