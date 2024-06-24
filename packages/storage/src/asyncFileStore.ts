import { IAsyncFileStore } from "./types";
import localforage from "localforage";
import { EventHub } from "@qstudio/utils";
import { IFileRenamedEvent, ProjectFilesEvent, ProjectFilesEventType } from "@qstudio/eventhubs";
import { filePathInFolder, getFoldersAndFiles, renameFolderInPath } from "./utils";
const FILE_PATHS_KEY = "~file_paths~";
const FILE_NAME_KEY = "~file~";



class AsyncFileStore implements IAsyncFileStore {
  keyPrefix: string;
  store: LocalForage;
  eventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  eventSource?: string;
  constructor(keyPrefix: string, store: LocalForage, eventHub?: EventHub<ProjectFilesEventType, ProjectFilesEvent>, eventSource?: string) {
    this.keyPrefix = keyPrefix;
    this.store = store;
    this.eventHub = eventHub || new EventHub<ProjectFilesEventType, ProjectFilesEvent>();
    this.createFolder = this.createFolder.bind(this);
    this.deleteFolder = this.deleteFolder.bind(this);
    this.renameFolder = this.renameFolder.bind(this);
    this.getFileContent = this.getFileContent.bind(this);
    this.getAllFiles = this.getAllFiles.bind(this);
    this.getFilePathKey = this.getFilePathKey.bind(this);
    this.getFilePathsKey = this.getFilePathsKey.bind(this);
    this.getFilePaths = this.getFilePaths.bind(this);
    this.ensureFilePaths = this.ensureFilePaths.bind(this);
    this.addFile = this.addFile.bind(this);
    this.addFiles = this.addFiles.bind(this);
    this.renameFile = this.renameFile.bind(this);
    this.renameFiles = this.renameFiles.bind(this);
    this.deleteFile = this.deleteFile.bind(this);
    this.deleteFiles = this.deleteFiles.bind(this);
    this.setFile = this.setFile.bind(this);
    this.eventSource = eventSource;
  }
  async ensureFile(path: string, defaultContent?: string | undefined): Promise<void> {

    const content = await this.store.getItem<string>(this.getFilePathKey(path));
    if (typeof content === "undefined" || content === null) {
      await this.addFile(path, defaultContent);
    }
  }
  async setFile(path: string, content: string): Promise<void> {
    await this.addFile(path, content);
  }
  async createFolder(folder: string): Promise<void> {
    await this.addFile(folder + "/.gitkeep");
  }
  async deleteFolder(folder: string): Promise<void> {
    const paths = await this.getFilePaths();
    const filesInFolder = paths.filter(x => filePathInFolder(x, folder));
    await this.deleteFiles(filesInFolder);
  }
  async renameFolder(oldPath: string, newPath: string): Promise<void> {
    const paths = await this.getFilePaths();
    const filesInFolder = paths.filter(x => filePathInFolder(x, oldPath));
    const targets = filesInFolder.map(x => ({oldPath: x, newPath: renameFolderInPath(x, oldPath, newPath)}));
    await this.renameFiles(targets);
  }
  async getFileContent(path: string): Promise<string> {
    const content = await this.store.getItem<string>(this.getFilePathKey(path));
    return content || "";
  }
  async getAllFiles(): Promise<{ path: string; content: string; }[]> {
    const paths = await this.getFilePaths();
    return Promise.all(paths.map(async path => ({path, content: (await this.store.getItem(this.getFilePathKey(path)))||""})));
  }
  getFilePathKey(path: string) {
    return `${this.keyPrefix}${FILE_NAME_KEY}${path}`;
  }
  getFilePathsKey() {
    return `${this.keyPrefix}${FILE_PATHS_KEY}`;
  }
  async getFilePaths() : Promise<string[]>{
    const paths: string[] | null = await this.store.getItem(this.getFilePathsKey());
    return getFoldersAndFiles(paths || []).files;
  }
  async ensureFilePaths(paths: string[]): Promise<boolean> {
    const currentPaths = await this.getFilePaths();
    const newPaths = paths.filter(x => !currentPaths.includes(x));
    if (newPaths.length > 0) {
      await this.store.setItem(this.getFilePathsKey(), currentPaths.concat(newPaths));
      newPaths.forEach(x => this.eventHub.notify(ProjectFilesEventType.FileCreated, {path: x, eventSource: this.eventSource}));
      return false;
    }else{
      return true;
    }
  }
  async addFile(path: string, content?: string) {
    const exists = await this.ensureFilePaths([path]);
    await this.store.setItem(this.getFilePathKey(path), content || "");
    if(exists){
      this.eventHub.notify(ProjectFilesEventType.FileModified, {path, eventSource: this.eventSource});
    }
  }
  async addFiles(files: {path: string, content?: string}[]) {
    await this.ensureFilePaths(files.map(x => x.path));
    await Promise.all(files.map(x => this.store.setItem(this.getFilePathKey(x.path), x.content || "")));
  }
  async renameFile(oldPath: string, newPath: string) {
    const content = await this.store.getItem(this.getFilePathKey(oldPath));
    await this.store.setItem(this.getFilePathKey(newPath), content);
    await this.store.removeItem(this.getFilePathKey(oldPath));
    const paths = await this.getFilePaths();
    await this.store.setItem(this.getFilePathsKey(), paths.map(x => x === oldPath ? newPath : x));
    this.eventHub.notify({type: ProjectFilesEventType.FileRenamed, path: oldPath, newPath, eventSource: this.eventSource});
  }
  async renameFiles(targets: {oldPath: string, newPath: string}[]) {
    const paths = await this.getFilePaths();
    await Promise.all(targets.map(async x => {
      const content = await this.store.getItem(this.getFilePathKey(x.oldPath));
      await this.store.setItem(this.getFilePathKey(x.newPath), content);
      await this.store.removeItem(this.getFilePathKey(x.oldPath));
    }));
    await this.store.setItem(this.getFilePathsKey(), paths.map(x => {
      const target = targets.find(t => t.oldPath === x);
      return target ? target.newPath : x;
    }));
    targets.forEach(x => this.eventHub.notify({type: ProjectFilesEventType.FileRenamed, path: x.oldPath, newPath: x.newPath, eventSource: this.eventSource}));
  }
  async deleteFile(path: string) {
    await this.store.removeItem(this.getFilePathKey(path));
    const paths = await this.getFilePaths();
    await this.store.setItem(this.getFilePathsKey(), paths.filter(x => x !== path));
    this.eventHub.notify(ProjectFilesEventType.FileDeleted, {path, eventSource: this.eventSource});
  }
  async deleteFiles(paths: string[]) {
    await Promise.all(paths.map(x => this.store.removeItem(this.getFilePathKey(x))));
    const currentPaths = await this.getFilePaths();
    await this.store.setItem(this.getFilePathsKey(), currentPaths.filter(x => !paths.includes(x)));
    paths.forEach(x => this.eventHub.notify(ProjectFilesEventType.FileDeleted, {path: x, eventSource: this.eventSource}));
  }
  withEventSource(eventSource: string): AsyncFileStore {
    return new AsyncFileStore(this.keyPrefix, this.store, this.eventHub, eventSource);
  }
}

export {
  AsyncFileStore,
}