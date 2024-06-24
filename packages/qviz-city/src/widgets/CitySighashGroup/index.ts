import {IBoundingBox, IQSRenderContext, IQVizStyleResolver, ITextElementHelper, ITreeJunctionLayout, QWTreeJunction, QWidget, multilineTextGroupV3, simpleStateDiff, } from '@qstudio/core';
import { makeSVGElemAttributes, makeSVGElement } from '@qstudio/qsvg';
import { ICitySighashGroth16ProofResult, getCircuitNameForJobIdHex, getCircuitWidgetNameForJobIdHex } from '@qstudio/city-block';
import { QWCityProof } from '../CityProof';

const WIDGET_TYPE_ID = "QWCitySighashGroup";

    
interface IQWCitySighashGroupConfig {
  jobs: ICitySighashGroth16ProofResult;
  junctionLayout: ITreeJunctionLayout;
}
interface IQWCitySighashGroupState {
}

type IQWCitySighashGroupStatePatch = Partial<IQWCitySighashGroupState>;
interface ICitySighashGroupElems {
  junction: QWTreeJunction;
  groth16Final: QWCityProof;
  sighashFinal: QWCityProof;
  sighashIntrospection: QWCityProof;
  stateTransitionLink: QWCityProof;
}
class QWCitySighashGroup extends QWidget<IQWCitySighashGroupConfig, IQWCitySighashGroupState, IQWCitySighashGroupStatePatch> {

  groupElems: ICitySighashGroupElems = null as any;

  getChildren(): QWidget<any, any, any>[] {
    return [
      this.groupElems.junction,
    ]
  }
  layoutInternal(childBBoxes: IBoundingBox[]): IBoundingBox {

    return this.groupElems.junction.getBBox();
  }
  getWidgetType(): string {
    return WIDGET_TYPE_ID;
  }
  getDefaultState(): IQWCitySighashGroupState {
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
  applyStateUpdate(stateUpdate: Partial<IQWCitySighashGroupState>): boolean {
    return false;
  }

  static create(config: IQWCitySighashGroupConfig): QWCitySighashGroup {
    const sighashIntrospection = new QWCityProof({jobId: config.jobs.sighash_introspection});
    const sighashFinal = new QWCityProof({jobId: config.jobs.sighash_final});
    const groth16Final = new QWCityProof({jobId: config.jobs.groth16_final});
    const stateTransitionLink = new QWCityProof({jobId: config.jobs.state_transition_reference});

    const lowerJunction = QWTreeJunction.create(sighashFinal, [sighashIntrospection, stateTransitionLink], {layout: config.junctionLayout});
    const junction = QWTreeJunction.create(groth16Final, [lowerJunction], {layout: config.junctionLayout});


    const widget = new QWCitySighashGroup(config);
    widget.groupElems = {
      junction,
      groth16Final,
      sighashFinal,
      sighashIntrospection,
      stateTransitionLink,
    };




 
    return widget;
  }
}

export type {
  IQWCitySighashGroupState,
}
export {
  QWCitySighashGroup,
}