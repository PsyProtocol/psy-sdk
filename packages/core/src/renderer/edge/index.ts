import { createPathElement, makeSVGElemAttributes, makeSVGElement } from "@qstudio/qsvg";
import { IResolvedSimpleEdgeDefinition } from "../../types/scene";

function renderSimpleEdge(edge: IResolvedSimpleEdgeDefinition): SVGGElement | SVGPathElement {

  const path = makeSVGElement("path", {
    className: edge.className || "q-simple-edge",
  }, {
    "stroke": edge.color || undefined,
    "stroke-width": edge.strokeWidth?(edge.strokeWidth+"px"):undefined,
    "stroke-dasharray": edge.strokeDasharray || undefined,
  });
  path.setAttributeNS(null, "d", `M ${edge.from.x} ${edge.from.y} L ${edge.to.x} ${edge.to.y}`);


  return path;
}

export {
  renderSimpleEdge,
}