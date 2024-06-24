import {IBoundingBox, IQSRenderContext, IQVizStyleResolver, ITextElementHelper, ITreeJunctionLayout, IVec2, QWTreeJunction, QWidget, multilineTextGroupV3, simpleStateDiff, v2m, } from '@qstudio/core';

const WIDGET_TYPE_ID = "QWCityBlockGroup";

    
interface IQWCityBlockGroupConfig {
}
interface IQWCityBlockGroupState {
}

type IQWCityBlockGroupStatePatch = Partial<IQWCityBlockGroupState>;
interface ICityBlockGroupElems {
  stateTransitionGroup: QWTreeJunction;
  sighashGroups: QWTreeJunction[];
}
class QWCityBlockGroup extends QWidget<IQWCityBlockGroupConfig, IQWCityBlockGroupState, IQWCityBlockGroupStatePatch> {

  groupElems: ICityBlockGroupElems = null as any;

  getChildren(): QWidget<any, any, any>[] {
    return [
      this.groupElems.stateTransitionGroup,
      ...this.groupElems.sighashGroups,
    ]
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {
    let spacingTopHorizontal = 100;
    const maxSpacingTopHorizontal = 220;
    const spacingBetween = 20;
    const sighashGroupSizes = this.groupElems.sighashGroups.map(x=>x.getBBox().size);
    const stateTransitionGroupSize = this.groupElems.stateTransitionGroup.getBBox().size;

    const totalWidthTopElems = sighashGroupSizes.reduce((acc, x)=>acc+x.width, 0);
    let totalWidthTop = totalWidthTopElems+(sighashGroupSizes.length-1)*spacingTopHorizontal;
    const totalWidthBottom = stateTransitionGroupSize.width;

    const totalWidth = Math.max(totalWidthTop, totalWidthBottom);
    const topHeight = Math.max(...sighashGroupSizes.map(x=>x.height));
    const totalHeight = stateTransitionGroupSize.height + spacingBetween + topHeight;

    const sighashGroupLocationY = -topHeight/2;
    let currentX = -totalWidth/2;
    if(sighashGroupSizes.length && totalWidthTop < totalWidthBottom){
      spacingTopHorizontal = Math.min(maxSpacingTopHorizontal, (totalWidthBottom-totalWidthTopElems)/(sighashGroupSizes.length-1));
      totalWidthTop = totalWidthTopElems+(sighashGroupSizes.length-1)*spacingTopHorizontal;
      const offsetX = (totalWidthBottom-totalWidthTop)/2;
      currentX = -totalWidth/2+offsetX;
    }
    const baseOffset = this.position;
    const sighashGroupBBoxes: IBoundingBox[] = [];
    for(let i=0;i<sighashGroupSizes.length;i++){
      sighashGroupBBoxes.push({
        center: v2m.add(baseOffset, {x: currentX+sighashGroupSizes[i].width/2, y: sighashGroupLocationY}),
        size: sighashGroupSizes[i],
      });
      currentX += sighashGroupSizes[i].width+spacingTopHorizontal;
    }

    const stateTransitionGroupBBox = {
      center: v2m.add(baseOffset, {x: 0, y: totalHeight-stateTransitionGroupSize.height/2}),
      size: stateTransitionGroupSize,
    };

    this.groupElems.sighashGroups.forEach((x, i)=>{
      x.position = sighashGroupBBoxes[i].center;
    });
    this.groupElems.stateTransitionGroup.position = stateTransitionGroupBBox.center;

    return {
      center: baseOffset,
      size: {
        width: totalWidth,
        height: totalHeight,
      },
    };
  }
  getWidgetType(): string {
    return WIDGET_TYPE_ID;
  }
  getDefaultState(): IQWCityBlockGroupState {
    return {
    };
  }
  renderInternal(
    context: IQSRenderContext,
    container: SVGGElement
  ): SVGGElement {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");

    return g;
  }
  applyStateUpdate(stateUpdate: Partial<IQWCityBlockGroupState>): boolean {
    return false;
  }

  static create(groupElems: ICityBlockGroupElems): QWCityBlockGroup {
    const widget = new QWCityBlockGroup({});
    widget.groupElems = groupElems;




 
    return widget;
  }
}

export type {
  IQWCityBlockGroupState,
}
export {
  QWCityBlockGroup,
}