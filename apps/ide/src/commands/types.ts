import { IDEContext } from "../utils/ideContext";

enum SelectActionType {
  NavigatePush = 0,
  NavigatePop = 1,
  NavigateReplace = 2,
  NavigateDepth = 3,
  NavigateHandler = 4,
  CustomAction = 5,
  CustomActionAsync = 6,

}
enum MenuOptionsType {
  Static = 0,
  Dynamic = 1,
}
enum MenuBarOptionRenderType {
  Standard = 0,
}

interface IMenuBarOptionRenderConfigBase {
  type: MenuBarOptionRenderType;
  className?: string;
}
interface IMenuBarOptionRenderConfigStandard extends IMenuBarOptionRenderConfigBase{
  type: MenuBarOptionRenderType.Standard;
  icon?: (props: any)=>JSX.Element;
  iconColor?: string;
  description?: string;
}
type IMenuBarOptionRenderConfig = IMenuBarOptionRenderConfigStandard;
interface ICommandBarOptionBase {
  id: string | number;
  actionType: SelectActionType;
  label: string;
  renderConfig: IMenuBarOptionRenderConfig;
  keywords?: string[];
  shortcuts?: string[];
  className?: string;
  value?: any;
}
type TMenuId = string | number;
interface ICommandBarFullState<M extends TMenuId> {
  ctx: IDEContext;
  searchText: string;
  state: ICommandBarMenu<M>[];
} 

interface ICommandBarSelection<M extends TMenuId> extends ICommandBarFullState<M> {
  selectedOption: ICommandBarOption<M>;
} 
type TNavigationUpdateHandler<M extends TMenuId> = (selection: ICommandBarSelection<M>, navigateTo: (state: ICommandBarMenu<M>[])=>void)=>any;
interface ICommandBarOptionNavigatePush<M extends TMenuId> extends ICommandBarOptionBase {
  actionType: SelectActionType.NavigatePush;
  subMenu: ICommandBarMenu<M> | ICommandBarMenu<M>[] | M | M[];
}
interface ICommandBarOptionNavigatePop extends ICommandBarOptionBase {
  actionType: SelectActionType.NavigatePop;
  popCount?: number;
}

interface ICommandBarOptionNavigateReplace<M extends TMenuId> extends ICommandBarOptionBase {
  actionType: SelectActionType.NavigateReplace;
  state: ICommandBarMenu<M> | ICommandBarMenu<M>[] | M | M[];
}

interface ICommandBarOptionNavigateDepth extends ICommandBarOptionBase {
  actionType: SelectActionType.NavigateDepth;
  depth: number;
}
interface ICommandBarOptionNavigateHandler<M extends TMenuId> extends ICommandBarOptionBase {
  actionType: SelectActionType.NavigateHandler;
  handler: TNavigationUpdateHandler<M>;
}
interface ICommandBarOptionCustomAction<M extends TMenuId> extends ICommandBarOptionBase {
  actionType: SelectActionType.CustomAction;
  handler: (selection: ICommandBarSelection<M>)=>(ICommandBarMenu<M> | ICommandBarMenu<M>[] | M | M[] | undefined | void);
}
interface ICommandBarOptionCustomActionAsync<M extends TMenuId> extends ICommandBarOptionBase {
  actionType: SelectActionType.CustomActionAsync;
  handler: (selection: ICommandBarSelection<M>)=>Promise<ICommandBarMenu<M> | ICommandBarMenu<M>[] | M | M[] | undefined | void>;
}

interface IOptionGroup<M extends TMenuId> {
  id: string;
  label: string;
  options: ICommandBarOption<M>[];
}
interface ICommandBarMenuBase<M extends TMenuId> extends ICommandBarOptionNavigatePush<M>{
  placeholder?: string;
  menuId: M;
  disableFilter?: boolean;
}

interface ICommandBarStaticMenu<M extends TMenuId> extends ICommandBarMenuBase<M> {
  optionsType: MenuOptionsType.Static;
  optionGroups: IOptionGroup<M>[];

}
interface ICommandBarDynamicMenu<M extends TMenuId> extends ICommandBarMenuBase<M> {
  optionsType: MenuOptionsType.Dynamic;
  optionGroupsGenerator: (selection: ICommandBarSelection<M>)=>IOptionGroup<M>[];
}
/*
interface ICommandBarAsyncDynamicMenu<M extends TMenuId> extends ICommandBarMenuBase<M> {
  optionGroupsGenerator: (selection: ICommandBarSelection<M>)=>Promise<IOptionGroup<M>[]>;
}
*/
type ICommandBarMenu<M extends TMenuId> = ICommandBarStaticMenu<M> | ICommandBarDynamicMenu<M>;// | ICommandBarAsyncDynamicMenu<M>;
type ICommandBarOption<M extends TMenuId> = ICommandBarOptionNavigatePush<M> | ICommandBarOptionNavigatePop | ICommandBarOptionNavigateReplace<M> | ICommandBarOptionNavigateDepth | ICommandBarOptionNavigateHandler<M> | ICommandBarOptionCustomAction<M> | ICommandBarOptionCustomActionAsync<M> | ICommandBarMenu<M>;
export {
  SelectActionType,
  MenuBarOptionRenderType,
  MenuOptionsType,
}
export type {
  ICommandBarOption,
  ICommandBarMenu,
  ICommandBarFullState,
  ICommandBarSelection,
  ICommandBarOptionBase,
  ICommandBarOptionNavigatePush,
  ICommandBarOptionNavigatePop,
  ICommandBarOptionNavigateReplace,
  ICommandBarOptionNavigateDepth,
  ICommandBarOptionNavigateHandler,
  ICommandBarOptionCustomAction,
  ICommandBarOptionCustomActionAsync,
  IMenuBarOptionRenderConfig,
  IMenuBarOptionRenderConfigStandard,

  IOptionGroup,
  TMenuId,
  TNavigationUpdateHandler,
}
