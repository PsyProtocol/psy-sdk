import { IDEMenuId } from "@qstudio/eventhubs";
import { getFileDisplayForFileName, getFileDisplayForFilePath, getFileExtForFilePath } from "../utils/fileIcons";
import { ICommandBarFullState, ICommandBarMenu, ICommandBarOption, ICommandBarOptionCustomAction, ICommandBarSelection, IOptionGroup, MenuBarOptionRenderType, MenuOptionsType, SelectActionType } from "./types";
import { uuidv4 } from "@qstudio/utils";

enum IDEMenuGroupId {
  IDEActions = "ide-actions",
  RunActions = "run-actions",
  Files = "files",
  Projects = "projects",
  
}

type TIDEMenuGenerator = {
  [E in IDEMenuId]: (selection: ICommandBarSelection<IDEMenuId>) => ICommandBarMenu<IDEMenuId>;
}


type TIDEGroupGenerator = {
  [E in IDEMenuGroupId]: (fullState: ICommandBarFullState<IDEMenuId>) => IOptionGroup<IDEMenuId>;
}

function resolveSelectedFile(fullState: ICommandBarFullState<IDEMenuId>) {
  if (fullState.state.length) {
    for (let i = 0; i < fullState.state.length; i++) {
      if (fullState.state[i].menuId === IDEMenuId.Files) {
        if (i !== fullState.state.length - 1 && typeof fullState.state[i + 1].value === 'string') {
          return fullState.state[i + 1].value;
        }else{
          return fullState.ctx.activeFile;
        }
      }
    }
  }
  return fullState.ctx.activeFile;

}
async function runJSFile(filePath: string, fullState: ICommandBarFullState<IDEMenuId>): Promise<void>{
  const content = await fullState.ctx.fileStorage.getFileContent(filePath);
  console.log("running js:\n"+content);
}
function generateRunOptionsForFileName(filePath: string, fullState: ICommandBarFullState<IDEMenuId>): ICommandBarOption<IDEMenuId>[] {
  const ext = getFileExtForFilePath(filePath);
  if(ext === "js"){
    return [
      {
        id: "run",
        label: "Run JavaScript File",
        actionType: SelectActionType.CustomActionAsync,
        handler: () => runJSFile(filePath, fullState),
        renderConfig: {type: MenuBarOptionRenderType.Standard},
      },
      
      {
        id: "debug",
        label: "Debug JavaScript File",
        actionType: SelectActionType.CustomActionAsync,
        handler: () => runJSFile(filePath, fullState),
        renderConfig: {type: MenuBarOptionRenderType.Standard},
      },
    ]
  }else{
    return [];
  }
}
interface IPromptMenuConfigBase {
  id: IDEMenuId;
  prompt: string;
  placeholder?: string;
  acceptOptionLabel: string;
  cancelOptionLabel?: string;
  isAsync: boolean;
}
interface IPromptMenuConfigSync extends IPromptMenuConfigBase {
  isAsync: false;
  acceptHandler: (selection: ICommandBarSelection<IDEMenuId>)=>void;
}
interface IPromptMenuConfigAsync extends IPromptMenuConfigBase {
  isAsync: true;
  acceptHandler: (selection: ICommandBarSelection<IDEMenuId>)=>Promise<void>;
}
type IPromptMenuConfig = IPromptMenuConfigSync | IPromptMenuConfigAsync;
function generatePromptMenu(config: IPromptMenuConfig): ICommandBarMenu<IDEMenuId> {
  const optionGroup = <IOptionGroup<IDEMenuId>>{
    id: "prompt",
    label: config.prompt,
    options: [
      {
        id: "accept",
        label: config.acceptOptionLabel,
        actionType: config.isAsync ? SelectActionType.CustomActionAsync : SelectActionType.CustomAction,
        handler: config.acceptHandler,
        renderConfig: {type: MenuBarOptionRenderType.Standard},

      },
    ]
  };
  
  if(config.cancelOptionLabel){
    optionGroup.options.push({
      id: "cancel",
      label: config.cancelOptionLabel,
      actionType: SelectActionType.NavigatePop,
      renderConfig: {type: MenuBarOptionRenderType.Standard},
    });
  }
  return {
    id: config.id,
    menuId: config.id,
    actionType: SelectActionType.NavigatePush,
    label: config.prompt,
    optionGroups: [optionGroup],
    subMenu: config.id,
    optionsType: MenuOptionsType.Static,
    renderConfig: {type: MenuBarOptionRenderType.Standard},
    disableFilter: true,
  }

}
const IDEGroupGenerators: TIDEGroupGenerator = {
  [IDEMenuGroupId.RunActions]: (fullState) => {
    const currentFilePath = resolveSelectedFile(fullState);
    const options = currentFilePath?generateRunOptionsForFileName(currentFilePath, fullState):[];

    return {
      id: IDEMenuGroupId.RunActions,
      label: "Run Actions",
      options: options,
    };
  },
  [IDEMenuGroupId.Files]: ({ctx}) => {
    const activeFile = ctx.activeFile;
    let files = ctx.fileExplorerConfig.getAllStandardFiles();

    const activeInd = files.indexOf(activeFile);
    if(activeInd !== -1){
      files = files.slice(activeInd).concat(files.slice(0, activeInd));
    }
    const fileOptions: ICommandBarOptionCustomAction<IDEMenuId>[] = files.map(filePath=>{
      const display = getFileDisplayForFilePath(filePath);
      return {
        id: filePath,
        label: display.name,
        actionType: SelectActionType.CustomAction,
        
        handler: () => {
          ctx.openFile(filePath);
        },
        renderConfig: {type: MenuBarOptionRenderType.Standard, description: display.parentFolder, icon: display.icon, iconColor: display.iconColor},
      }
    });
    return {
      id: IDEMenuGroupId.Files,
      label: "Files",
      options: fileOptions,
    }
  },
  [IDEMenuGroupId.Projects]: ({ctx}) => {
    const projects = ctx.projectManager.projects;
    const projectOptions: ICommandBarOption<IDEMenuId>[] = projects.map(project=>{
      return {
        id: project.id,
        label: project.name,
        actionType: SelectActionType.CustomAction,
        handler: () => {
          ctx.projectManager.openProject(project.id);
        },
        renderConfig: {type: MenuBarOptionRenderType.Standard, description: new Date(project.lastOpenedAt).toLocaleString()},
      }
    });
    projectOptions.push(
      {
        id: "new-project",
        label: "New Project...",
        actionType: SelectActionType.NavigateReplace,
        state: IDEMenuId.NewProject,
        renderConfig: {type: MenuBarOptionRenderType.Standard},
      }
    );
    return {
      id: IDEMenuGroupId.Projects,
      label: "Projects",
      options: projectOptions,
    }
  },
  [IDEMenuGroupId.IDEActions]: ({ctx}) => {
    return {
      id: IDEMenuGroupId.IDEActions,
      label: "IDE Actions",
      options: [
        {
          id: "rename-project",
          label: "Rename Project...",
          actionType: SelectActionType.NavigateReplace,
          state: IDEMenuId.RenameProject,
          renderConfig: {type: MenuBarOptionRenderType.Standard},
        },
        {
          id: "new-project",
          label: "New Project...",
          actionType: SelectActionType.NavigateReplace,
          state: IDEMenuId.NewProject,
          renderConfig: {type: MenuBarOptionRenderType.Standard},
        },
        {
          id: "open-project",
          label: "Open Project",
          actionType: SelectActionType.NavigateReplace,
          state: IDEMenuId.Projects,
          renderConfig: {type: MenuBarOptionRenderType.Standard},
        },
      ]
    }
  },

}

const IDEMenuGenerators: TIDEMenuGenerator = {
  [IDEMenuId.Standard]: (selection: ICommandBarSelection<IDEMenuId>) => {
    return <ICommandBarMenu<IDEMenuId>>{
      id: IDEMenuId.Standard,
      menuId: IDEMenuId.Standard,
      actionType: SelectActionType.NavigatePush,
      label: "IDE",
      optionGroups: [
        IDEGroupGenerators[IDEMenuGroupId.RunActions](selection),
        IDEGroupGenerators[IDEMenuGroupId.IDEActions](selection),
        IDEGroupGenerators[IDEMenuGroupId.Files](selection),
      ],
      subMenu: IDEMenuId.Standard,
      renderConfig: {type: MenuBarOptionRenderType.Standard},
      optionsType: MenuOptionsType.Static,
    };
  },
  [IDEMenuId.Files]: function (selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId> {
    return <ICommandBarMenu<IDEMenuId>>{
      id: IDEMenuId.Files,
      menuId: IDEMenuId.Files,
      actionType: SelectActionType.NavigatePush,
      label: "Files",
      optionGroups: [
        IDEGroupGenerators[IDEMenuGroupId.Files](selection),
      ],
      subMenu: IDEMenuId.Files,
      renderConfig: {type: MenuBarOptionRenderType.Standard},
      optionsType: MenuOptionsType.Static,
    };

  },
  [IDEMenuId.Projects]: function (selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId> {
    return <ICommandBarMenu<IDEMenuId>>{
      id: IDEMenuId.Projects,
      menuId: IDEMenuId.Projects,
      actionType: SelectActionType.NavigatePush,
      label: "Projects",
      optionGroups: [
        IDEGroupGenerators[IDEMenuGroupId.Projects](selection),
      ],
      subMenu: IDEMenuId.Projects,
      renderConfig: {type: MenuBarOptionRenderType.Standard},
      optionsType: MenuOptionsType.Static,
    };
  },
  [IDEMenuId.NewProject]: function (selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId> {
    return generatePromptMenu({
      id: IDEMenuId.NewProject,
      prompt: "New Project...",
      placeholder: "Project Name...",
      acceptOptionLabel: "Create Project",
      cancelOptionLabel: "Cancel",
      isAsync: true,
      acceptHandler: async (selection) => {
        const name = selection.searchText.trim();
        if(name.length){
          selection.ctx.projectManager.createProject(name, true);
        }
      },
    })
  },
  [IDEMenuId.RenameProject]: function (selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId> {
    return generatePromptMenu({
      id: IDEMenuId.NewProject,
      prompt: "Rename Project",
      placeholder: "New Project Name...",
      acceptOptionLabel: "Rename Project",
      cancelOptionLabel: "Cancel",
      isAsync: true,
      acceptHandler: async (selection) => {
        if(selection.searchText.trim().length){
          const activeProject  =selection.ctx.projectManager.activeProject;
          if(activeProject){
            selection.ctx.projectManager.updateProject({...activeProject, name: selection.searchText.trim()});
          }
        }
      },
    })
  }
};

function resolveMenuStateSingle(menuGenerator: TIDEMenuGenerator, value: ICommandBarMenu<IDEMenuId> | IDEMenuId, selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId>{
  if(typeof value === 'object' && value){
    return value;
  }else{
    return menuGenerator[value](selection);
  }
}

function resolveMenuState(menuGenerator: TIDEMenuGenerator, value: ICommandBarMenu<IDEMenuId> | ICommandBarMenu<IDEMenuId>[] | IDEMenuId | IDEMenuId[] | undefined | void, selection: ICommandBarSelection<IDEMenuId>): ICommandBarMenu<IDEMenuId>[]{
  if(typeof value === 'undefined' || value === null){
    return [];
  }
  const realValue = Array.isArray(value) ? value : [value];
  return realValue.map((v)=>resolveMenuStateSingle(menuGenerator, v, selection));
}


export {
  IDEMenuGenerators,
  resolveMenuState,
}

export type {
  TIDEMenuGenerator,
}