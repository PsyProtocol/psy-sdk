import {
  IQSRenderContext,
  ITextElementHelper,
  multilineTextGroupV3,
} from "@qstudio/core";
import { makeSVGElemAttributes, makeSVGElement } from "@qstudio/qsvg";
import { IQVCityProofNodeElems, IQVCityProofStyleDef } from "./types";


const QWCityProofRectWidth = 228;
const QWCityProofRectHeight = 160;

function getNodeElems(
  context: IQSRenderContext,
  style: IQVCityProofStyleDef
): IQVCityProofNodeElems {
  const base = makeSVGElemAttributes<SVGGElement>("g." + style.base);

  const borderRect = makeSVGElement(
    "rect",
    {},
    {
      fill: "transparent",
      class: style.borderRect,
      width: QWCityProofRectWidth,
      height: QWCityProofRectHeight,
      x: 0,
      y: 0,
    }
  );

  const label = multilineTextGroupV3(
    context.measurePaper,
    { x: QWCityProofRectWidth / 2, y: 56 },
    { class: style.label },
    1.75
  );
  base.appendChild(label.g);
  const statusRect = makeSVGElement(
    "rect",
    {},
    {
      class: style.statusRect,
      width: QWCityProofRectWidth,
      height: 30,
      x: 0,
      y: 0,
    }
  );
  const statusText = multilineTextGroupV3(
    context.measurePaper,
    { x: QWCityProofRectWidth / 2, y: 16 },
    { class: style.statusText }
  );

  const statusGroup = makeSVGElement("g", {}, { class: style.statusGroup }, [
    statusRect,
    statusText.g,
  ]);
  base.appendChild(statusGroup);
  base.appendChild(borderRect);

  const outerGroup = makeSVGElement(
    "g",
    {},
    {
      transform: `translate(${-QWCityProofRectWidth / 2}, ${
        -QWCityProofRectHeight / 2
      })`,
      class: style.outerGroup,
    },
    [base]
  );

  return {
    base,
    borderRect,
    label,
    statusGroup,
    statusRect,
    statusText,
    outerGroup,
  };
}

export {
  getNodeElems,
}
