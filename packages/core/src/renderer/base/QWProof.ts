import { NodeAnchor } from "../../anchor";
import { simpleStateDiff } from "../../diff";
import { IBoundingBox } from "../../types/geo";
import { IQSRenderContext } from "../../types/renderer";
import { multilineTextGroupV3 } from "../text/helpers";
import { QWidget } from "./QWidget";
import { makeSVGElemAttributes, makeSVGElement } from "@qstudio/qsvg";
const QWProofRectWidth = 228;
const QWProofRectHeight = 160;

interface IProofNodeElems {
  group: SVGGElement;
  outerGroup: SVGGElement;
  borderRect: SVGRectElement;
  label: {
    g: SVGGraphicsElement;
    setText: (str: string) => void;
  };
  statusGroup: SVGGElement;
  statusRect: SVGRectElement;
  statusText: {
    g: SVGGraphicsElement;
    setText: (str: string) => void;
  };
}
function createProofNodeElems(context: IQSRenderContext): IProofNodeElems {
  const base = makeSVGElemAttributes<SVGGElement>("g.qProofWidget");


  const borderRect = makeSVGElement("rect", {}, {
    fill: "transparent",
    "class": "qProofBorderRect",
    stroke: "#D8D129",
    width: QWProofRectWidth,
    height: QWProofRectHeight,
    x: 0,
    y: 0,
    style: "transition: stroke 500ms",
  });

  const label = multilineTextGroupV3(context.measurePaper, {x: QWProofRectWidth/2, y: 56}, {"class": "qProofLabel"}, 1.75);
  base.appendChild(label.g);
  const statusRect = makeSVGElement("rect", {}, {
    fill: "transparent",
    "class": "qProofStatusRect",
    width: QWProofRectWidth,
    height: 30,
    x: 0,
    y: 0,
    style: "transition: stroke 500ms",
  });
  const statusText = multilineTextGroupV3(context.measurePaper, {x: QWProofRectWidth/2, y: 16}, {"class": "qProofStatusText"});

  const statusGroup = makeSVGElement("g", {}, {"class": "qProofStatusGroup"}, [
    statusRect,
    statusText.g,
  ]);
  base.appendChild(statusGroup);
  base.appendChild(borderRect);

  const outerGroup = makeSVGElement("g", {}, {"transform": `translate(${-QWProofRectWidth/2}, ${-QWProofRectHeight/2})`}, [
    base,
  ]);

  return {
    group: base,
    borderRect,
    label,
    statusGroup,
    statusRect,
    statusText,
    outerGroup,
  };

}

interface IQWProofConfig {
  className?: string;

}
interface IQWProofState {
  label: string;
  state: "waiting" | "proving" | "proved";
  statusMessage: string;
  jobId: string;
}

type IQWProofStatePatch = Partial<IQWProofState>;

class QWProof extends QWidget<IQWProofConfig, IQWProofState, IQWProofStatePatch> {
  getChildren(): QWidget<any, any, any>[] {
    return [];
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    return {
      center: this.position,
      size: {
        width: QWProofRectWidth,
        height: QWProofRectHeight,
      },
    };
  }
  getWidgetType(): string {
    return "QWProof";
  }

  elems?: IProofNodeElems;
  
  getDefaultState(): IQWProofState {
    return {
      label: "Token\nTransfer",
      state: "waiting",
      statusMessage: "Waiting for Prover...",
      jobId: "fffff",
    };
  }
  renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const baseB = createProofNodeElems(context);
    this.elems = baseB;
    container.appendChild(baseB.outerGroup);

    baseB.label.setText(this.state.label);
    baseB.statusText.setText(this.state.statusMessage);
    container.removeChild(baseB.outerGroup);
    return baseB.outerGroup;
  }
  applyStateUpdate(stateUpdate: Partial<IQWProofState>): boolean {
    const {newState, diff, changed} = simpleStateDiff(this.state, stateUpdate);
    if(!changed || !this.elems){
      return false;
    }
    this.state = newState;
    if(diff.label){
      this.elems.label.setText(diff.label);
    }
    if(diff.statusMessage){
      this.elems.statusText.setText(diff.statusMessage);
    }
    if(diff.state){
      if(diff.state === "waiting"){
        this.elems.borderRect.setAttribute("stroke", "#D8D129");
        this.elems.statusRect.setAttribute("stroke", "#D8D129");
      }else if(diff.state === "proving"){
        this.elems.borderRect.setAttribute("stroke", "#FFA500");
        this.elems.statusRect.setAttribute("stroke", "#FFA500");
      }else if(diff.state === "proved"){
        this.elems.borderRect.setAttribute("stroke", "#0f0");
        this.elems.statusRect.setAttribute("stroke", "#0f0");
      }
    }
    return false;
  }
}


export {
  QWProof,
}