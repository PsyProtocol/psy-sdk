import localforage from "localforage";
import { AsyncGlobalKVStore } from "@qstudio/storage";
import { IProjectMetaData } from "./projects/types";
import { EventHub, uuidv4 } from "@qstudio/utils";
import { validate as isUUID } from 'uuid';
import { IDEContext } from "./ideContext";
import { EditorUIEventType, IEditorUIEvent } from "@qstudio/eventhubs";
const PROJECT_LIST_KEY = "studio-project-list-v1";
const CURRENT_PROJECT_SCHEMA_VERSION = "0.1.0";
const compareProjects = (a: IProjectMetaData, b: IProjectMetaData) => a.id === b.id;

class GlobalProjectManager {
  activeProject?: IProjectMetaData;
  activeIDEContext?: IDEContext;
  projects: IProjectMetaData[] = [];
  store: AsyncGlobalKVStore;
  uiEventHub: EventHub<EditorUIEventType, IEditorUIEvent> = new EventHub();

  constructor(store: AsyncGlobalKVStore){
    this.store = store;
  }

  async refreshProjects(){
    const projects = await this.store.getItem<IProjectMetaData[]>(PROJECT_LIST_KEY);
    if(projects){
      this.projects = projects;
    }else{
      this.projects = [];
    }
  }
  async updateProject(project: IProjectMetaData){
    this.projects = await this.store.addToSet<IProjectMetaData>(PROJECT_LIST_KEY, project, compareProjects, true);
    if(this.activeProject?.id === project.id){
      this.activeProject = project;
    }
  }

  async openProject(projectId: string): Promise<IProjectMetaData> {
    if(this.activeProject?.id === projectId){
      return this.activeProject;
    }else{
      await this.refreshProjects();
      const project = this.projects.filter(p => p.id === projectId)[0];
      if(project){
        project.lastOpenedAt = Date.now();
        await this.updateProject(project);
        this.activeProject = project;
        this.activeIDEContext = await IDEContext.newContext(this);
        this.uiEventHub.notify({type: EditorUIEventType.OpenProject, projectId: project.id});
        return project;
      }else{
        throw new Error("Project not found");
      }
    }
  }
  async createProject(name: string, open?: boolean){
    const project: IProjectMetaData = {
      id: uuidv4(),
      name,
      createdAt: Date.now(),
      lastOpenedAt: Date.now(),
      schemaVersion: CURRENT_PROJECT_SCHEMA_VERSION,
    };
    await this.updateProject(project);
    if(open){
      this.activeProject = project;
      this.activeIDEContext = await IDEContext.newContext(this);
      this.uiEventHub.notify({type: EditorUIEventType.OpenProject, projectId: project.id});
    }
    return project;
  }
  hasProjectId(id: string): boolean{
    return this.projects.filter(p => p.id === id).length === 1;
  }





  static async init(projectId?: string): Promise<GlobalProjectManager> {
    const localForageInstance = localforage.createInstance({name: "STUDIO_DEMO_PROJECTS"});
    const store = new AsyncGlobalKVStore("STUDIO_DEMO_PROJECTS", localForageInstance);
    const manager = new GlobalProjectManager(store);
    await manager.refreshProjects();
    if(manager.projects.length === 0){
      await manager.createProject("Demo Project", true);
    }else{
      if(projectId && manager.hasProjectId(projectId)){
        await manager.openProject(projectId);
      }else{
        await manager.openProject(manager.projects[0].id);
      }
    }
    return manager;
  }




}

export {
  GlobalProjectManager,
}