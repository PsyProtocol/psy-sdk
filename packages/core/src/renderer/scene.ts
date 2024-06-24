import { NodeAnchor, TNodeAnchor } from "../anchor";
import { QVizStyleResolver } from "../styleResolver";
import { IVec2 } from "../types/geo";
import { IQSRenderContext, IQStudioSceneManager } from "../types/renderer";
import { getOffsetPointFromAnchor } from "../vecmath/bbox";
import { anchorUV } from "../vecmath/vec2";
import { QWidget } from "./base/QWidget";
import { QEDVizPaper } from "./vizpaper";

class QSceneManager implements IQStudioSceneManager{
  vizPaper: QEDVizPaper;
  widgetMap: Record<string, QWidget<any, any, any>> = {};
  styleResolver: QVizStyleResolver;
  constructor(vizPaper: QEDVizPaper, styleResolver?: QVizStyleResolver) {
    this.vizPaper = vizPaper;
    this.styleResolver = styleResolver || new QVizStyleResolver();
    this.forceResize = this.forceResize.bind(this);
    this.dispose = this.dispose.bind(this);
  }
  getStyleDef<T, C>(widgetId: string, config: C): T {
    return this.styleResolver.getStyleDef<T, C>(widgetId, config);
  }
  getRenderContext(): IQSRenderContext {
    return {
      paper: this.vizPaper.root,
      measurePaper: this.vizPaper.root,
      manager: this,
      svg: this.vizPaper.svg.node,
    };
  }
  getWidgetAnchorPoint(widget: string | QWidget<any, any, any>, anchor: TNodeAnchor, offset = {x: 0, y: 0}): IVec2 | null {
    let w = typeof widget === 'string' ? this.widgetMap[widget] : widget;
    if(!w){
      return null;
    }
    const bbox = {size: {width: 200, height: 200}, center: {x: 0, y: 0}};
    /*console.log("test bbox: ", bbox);
    console.log("test anchor results: ",JSON.stringify(Object.keys(NodeAnchor).map(k=>({name: k, anchor: (NodeAnchor as any)[k], uv: anchorUV((NodeAnchor as any)[k]), point: getOffsetPointFromAnchor(bbox, (NodeAnchor as any)[k], {x: 0, y: 0})})), null, 2));
    */
    return getOffsetPointFromAnchor(w.getBBox(), anchor, offset);
  }
  updateWidgetState<U = any>(id: string, stateUpdate: U): boolean {
    const widget = this.widgetMap[id];
    if (widget) {
      widget.updateState(stateUpdate);
      return true;
    }else{
      return false;
    }
  }
  getWidget<W extends QWidget<C, S, U>, C, S, U>(id: string): W | null {
    if(this.widgetMap[id]){
      return this.widgetMap[id] as W;
    }else{
      return null;
    }
  }
  addWidget<W extends QWidget<C, S, U>, C = any, S = any, U = any>(widget: W): W {
    widget.getChildren().forEach(child => this.addWidget(child));
    this.widgetMap[widget.id] = widget;
    return widget;
  }
  setVizPaper(paper: QEDVizPaper) {
    this.vizPaper = paper;
    this.forceResize();
  }

  forceResize() {
    this.vizPaper.resizeToFit();
  }
  dispose() {
    this.vizPaper.dispose();
  }
}

export {
  QSceneManager,
}