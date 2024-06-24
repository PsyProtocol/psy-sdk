import { EventHub } from "@qstudio/utils";
import { IAsyncFileStore, ISyncFileStore } from "./types";
import { ProjectFilesEvent, ProjectFilesEventType } from "@qstudio/eventhubs";
import { filePathInFolder, renameFolderInPath } from "./utils";
interface ISyncStore {
  getItem<T>(key: string): T | null;
  setItem<T>(key: string, value: T): void;
  removeItem(key: string): void;
}
class SimpleMemorySyncStore implements ISyncStore {
  removeItem(key: string): void {
    delete this.store[key];
  }
  store: Record<string, any> = {};
  getItem<T>(key: string): T | null {
    return this.store[key] || null;
  }
  setItem<T>(key: string, value: T): void {
    this.store[key] = value;
  }
}

const FILE_PATHS_KEY = "~file_paths~";
const FILE_NAME_KEY = "~file~";


class MemoryFileStore implements ISyncFileStore {
  store: ISyncStore;
  keyPrefix: string;
  eventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  eventSource?: string;
  
  constructor(keyPrefix: string, eventHub?: EventHub<ProjectFilesEventType, ProjectFilesEvent>, eventSource?: string) {
    this.store = new SimpleMemorySyncStore();
    this.keyPrefix = keyPrefix;
    this.eventHub = eventHub || new EventHub<ProjectFilesEventType, ProjectFilesEvent>();
  }
  setFile(path: string, content: string): void {
    this.addFile(path, content);
  }
  async loadFromAsyncStore(asyncStore: IAsyncFileStore): Promise<void> {
    await asyncStore.getAllFiles().then(files => {
      this.addFiles(files);
    });
  }
  ensureFile(path: string, defaultContent?: string | undefined) {
    const content = this.store.getItem<string>(this.getFilePathKey(path));
    if (typeof content === "undefined" || content === null) {
      this.addFile(path, defaultContent);
    }
  }

  createFolder(folder: string): void {
    this.addFile(folder + "/.gitkeep");
  }
  deleteFolder(folder: string): void {
    const paths = this.getFilePaths();
    const filesInFolder = paths.filter(x => filePathInFolder(x, folder));
    this.deleteFiles(filesInFolder);
  }
  renameFolder(oldPath: string, newPath: string): void {
    const paths = this.getFilePaths();
    const filesInFolder = paths.filter(x => filePathInFolder(x, oldPath));
    const targets = filesInFolder.map(x => ({oldPath: x, newPath: renameFolderInPath(x, oldPath, newPath)}));
    this.renameFiles(targets);
  }
  getFileContent(path: string): string {
    const content = this.store.getItem<string>(this.getFilePathKey(path));
    return content || "";
  }
  getAllFiles(): { path: string; content: string; }[] {
    const paths = this.getFilePaths();
    return (paths.map(path => ({path, content: (this.store.getItem(this.getFilePathKey(path)))||""})));
  }
  getFilePathKey(path: string) {
    return `${this.keyPrefix}${FILE_NAME_KEY}${path}`;
  }
  getFilePathsKey() {
    return `${this.keyPrefix}${FILE_PATHS_KEY}`;
  }
  getFilePaths() : string[]{
    const paths: string[] | null = this.store.getItem(this.getFilePathsKey());
    return paths || [];
  }
  ensureFilePaths(paths: string[]) {
    const currentPaths = this.getFilePaths();
    const newPaths = paths.filter(x => !currentPaths.includes(x));
    if (newPaths.length > 0) {
      this.store.setItem(this.getFilePathsKey(), currentPaths.concat(newPaths));
      newPaths.forEach(x => this.eventHub.notify(ProjectFilesEventType.FileCreated, {path: x, eventSource: this.eventSource}));
      return false;
    }else{
      return true;
    }
  }
  addFile(path: string, content?: string) {
    const exists = this.ensureFilePaths([path]);
    this.store.setItem(this.getFilePathKey(path), content || "");
    if(exists){
      this.eventHub.notify(ProjectFilesEventType.FileModified, {path, eventSource: this.eventSource});
    }
  }
  addFiles(files: {path: string, content?: string}[]) {
    this.ensureFilePaths(files.map(x => x.path));
    (files.map(x => this.store.setItem(this.getFilePathKey(x.path), x.content || "")));
  }
  renameFile(oldPath: string, newPath: string) {
    const content = this.store.getItem(this.getFilePathKey(oldPath));
    this.store.setItem(this.getFilePathKey(newPath), content);
    this.store.removeItem(this.getFilePathKey(oldPath));
    const paths = this.getFilePaths();
    this.store.setItem(this.getFilePathsKey(), paths.map(x => x === oldPath ? newPath : x));
    this.eventHub.notify({type: ProjectFilesEventType.FileRenamed, path: oldPath, newPath, eventSource: this.eventSource});
  }
  renameFiles(targets: {oldPath: string, newPath: string}[]) {
    const paths = this.getFilePaths();
    (targets.map((x) => {
      const content = this.store.getItem(this.getFilePathKey(x.oldPath));
      this.store.setItem(this.getFilePathKey(x.newPath), content);
      this.store.removeItem(this.getFilePathKey(x.oldPath));
    }));
    this.store.setItem(this.getFilePathsKey(), paths.map(x => {
      const target = targets.find(t => t.oldPath === x);
      return target ? target.newPath : x;
    }));
    targets.forEach(x => this.eventHub.notify({type: ProjectFilesEventType.FileRenamed, path: x.oldPath, newPath: x.newPath, eventSource: this.eventSource}));
  }
  deleteFile(path: string) {
    this.store.removeItem(this.getFilePathKey(path));
    const paths = this.getFilePaths();
    this.store.setItem(this.getFilePathsKey(), paths.filter(x => x !== path));
    this.eventHub.notify(ProjectFilesEventType.FileDeleted, {path, eventSource: this.eventSource});
  }
  deleteFiles(paths: string[]) {
    (paths.map(x => this.store.removeItem(this.getFilePathKey(x))));
    const currentPaths = this.getFilePaths();
    this.store.setItem(this.getFilePathsKey(), currentPaths.filter(x => !paths.includes(x)));
    paths.forEach(x => this.eventHub.notify(ProjectFilesEventType.FileDeleted, {path: x, eventSource: this.eventSource}));
  } 
  withEventSource(eventSource: string): MemoryFileStore {
    const memFs= new MemoryFileStore(this.keyPrefix, this.eventHub, eventSource);
    memFs.store = this.store;
    return memFs;
  }
  
}

export {
  MemoryFileStore,
}