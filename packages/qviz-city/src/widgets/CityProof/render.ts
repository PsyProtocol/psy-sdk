import {
  IQSRenderContext,
  ISize,
  ITextElementHelper,
  IVec2,
  multilineTextGroupV3,
} from "@qstudio/core";
import { makeSVGElemAttributes, makeSVGElement } from "@qstudio/qsvg";
import { ICityProofInfoStore, ICityProofStyleState, IQVCityProofNodeElems, IQVCityProofStyleDef } from "./types";
import { ProofWidgetStyleVariant } from "./types";

const QWCityProofRectWidth = 228;
const QWCityProofRectHeight = 160;
const QWCityProofTotalHeight = 212;
const QWCityProofStatusHeight = 36;
const QWCityProofStatusMarginTop = 12;

interface IProofWidgetVariantConfig {
  proofRectSize: ISize;
  labelPosition: IVec2;
  iconHeight: number;
  iconPosition: IVec2;
}
const STANDARD_VARIANT: IProofWidgetVariantConfig = {
  proofRectSize: { width: 228, height: 160 },
  labelPosition: { x: 114, y: 32 },
  iconHeight: 90,
  iconPosition: { x: 114, y: 56 },
};

const AGGREGATE_VARIANT: IProofWidgetVariantConfig = {
  proofRectSize: { width: 228, height: 184 },
  labelPosition: { x: 114, y: 36 },
  iconHeight: 90,
  iconPosition: { x: 114, y: 56+24 },
};
const BIG_PADDING = 16; 
const BIG_AGGREGATE_VARIANT: IProofWidgetVariantConfig = {
  proofRectSize: { width: 297+2*BIG_PADDING, height: 145+2*BIG_PADDING },
  labelPosition: { x: 0xffffffff, y: 0 },
  iconHeight: 145,
  iconPosition: { x: 297/2+BIG_PADDING, y:BIG_PADDING },
};
const NO_ICON_VARIANT: IProofWidgetVariantConfig = {
  proofRectSize: { width: 228, height: 160 },
  labelPosition: { x: 114, y: 32 },
  iconHeight: 0,
  iconPosition: { x: 114, y: 56 },
};
const VARIANT_MAP: Record<ProofWidgetStyleVariant, IProofWidgetVariantConfig> = {
  [ProofWidgetStyleVariant.Standard]: STANDARD_VARIANT,
  [ProofWidgetStyleVariant.Aggregate]: AGGREGATE_VARIANT,
  [ProofWidgetStyleVariant.BigAggregate]: BIG_AGGREGATE_VARIANT,
  [ProofWidgetStyleVariant.TextOnly]: NO_ICON_VARIANT,
};
/*


  const isAggregation = styleState.labelText.indexOf("[AGG]\n") !== -1;
  const realLabel = isAggregation?("Aggregate\n"+styleState.labelText.substring(6)):styleState.labelText;
  const isMultiLine = !isAggregation && realLabel.includes("\n");
  console.log("isAggregation", isAggregation, "isMultiLine", isMultiLine, "realLabel", realLabel);

  */
function getNodeElems(
  context: IQSRenderContext,
  style: IQVCityProofStyleDef,
  styleState: ICityProofStyleState,
  icon: SVGGElement,
  iconSize: ISize,
  variant: ProofWidgetStyleVariant = ProofWidgetStyleVariant.Standard,
): IQVCityProofNodeElems {
  const vs = VARIANT_MAP[variant];
  const base = makeSVGElemAttributes<SVGGElement>("g." + style.base);
    const borderRect = makeSVGElement(
    "rect",
    {},
    {
      fill: "transparent",
      class: style.borderRect,
      width: vs.proofRectSize.width,
      height: vs.proofRectSize.height,
      x: 0,
      y: 0,
      rx: 16,
    }
  );
  icon.setAttribute("transform-origin", `${iconSize.width/2} ${iconSize.height/2}`);
  icon.setAttribute("transform", `translate(${vs.iconPosition.x-iconSize.width/2}, ${vs.iconPosition.y})`);
  base.appendChild(icon);

  const hasLabel = vs.labelPosition.x !== 0xffffffff;

  const label = hasLabel?multilineTextGroupV3(
    context.measurePaper,
    { x: vs.labelPosition.x, y: vs.labelPosition.y },
    { class: style.label },
    1.75
  ):{g:document.createElementNS("http://www.w3.org/2000/svg", "g"), setText:()=>{}};
  label.setText(styleState.labelText);

  base.appendChild(label.g);
  const statusRect = makeSVGElement(
    "rect",
    {},
    {
      class: style.statusRect,
      width: vs.proofRectSize.width,
      height: 36,
      x: 0,
      y: 0,
      rx: 16,
    }
  );
  const statusText = multilineTextGroupV3(
    context.measurePaper,
    { x: vs.proofRectSize.width / 2, y: 20 },
    { class: style.statusText }
  );
  statusText.setText(styleState.statusText);

  const statusGroup = makeSVGElement("g", {}, { class: style.statusGroup }, [
    statusRect,
    statusText.g,
  ]);
  base.appendChild(statusGroup);
  base.appendChild(borderRect);
  statusGroup.setAttributeNS(null, "transform", `translate(0, ${vs.proofRectSize.height+QWCityProofStatusMarginTop})`);

  const outerGroup = makeSVGElement(
    "g",
    {},
    {
      transform: `translate(${-QWCityProofRectWidth / 2}, ${
        -QWCityProofTotalHeight / 2
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
  QWCityProofTotalHeight,
  QWCityProofRectWidth,
}
