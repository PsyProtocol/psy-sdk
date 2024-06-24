/*import { ISize } from "./geo";

interface IQSElementDefintion {
  id: string;
  template: string;
  css: string;
  render: (state: any) => SVGGElement;
  size: ISize;
}
interface IQSSerializedElementDefintion {
  id: string;
  template: string;
  css: string;
}

interface IQSElementInstance {
  id: string;
  element: string;
  state?: any;
  children: IQSElementInstance[];
}


interface IQStudioSceneBase {
  id: string;
  name: string;
  root: IQSElementInstance;
}

interface IQSerializedStudioSceneSerialized extends IQStudioSceneBase {
  definition: IQSSerializedElementDefintion[];
}
interface IQSerializedStudioScene extends IQStudioSceneBase {
  definition: IQSElementDefintion[];
}*/

type OldSceneLeftOver = boolean;

export type {
  OldSceneLeftOver
};