import { TNodeAnchor } from "../anchor";
import { QWidget } from "../renderer";
import { IQVizStyleResolver } from "../styleResolver";
import { IVec2 } from "./geo";
import { IQWidgetSerialized, IQWidgetSerializedCore } from "./scene";

interface IQSRenderContext {
  svg: SVGSVGElement;
  paper: SVGGElement;
  measurePaper: SVGGElement;
  manager: IQStudioSceneManager;
}

interface IQSWidgetDeserializer {
  deserializeWidget<C, S, U>(serialized: IQWidgetSerialized<C, S>): QWidget<C, S, U>;
}

interface IQWidgetRenderHelper extends IQVizStyleResolver {
  getWidget<W extends QWidget<C, S, U>, C, S, U>(id: string): W | null;
  getWidgetAnchorPoint(widget: string | QWidget<any, any, any>, anchorPoint: TNodeAnchor, offset?: IVec2): IVec2 | null;


}
interface IQStudioSceneManager extends IQWidgetRenderHelper {
  updateWidgetState<U = any>(id: string, stateUpdate: U): boolean;

}


export type {
  IQSRenderContext,
  IQStudioSceneManager,
  IQSWidgetDeserializer,
}