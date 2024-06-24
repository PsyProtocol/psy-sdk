import { AsyncFileStore } from "./asyncFileStore";
import { MemoryFileStore } from "./memoryFileStore";
import { IAsyncFileStore, ISyncFileStore } from "./types";
import { createDebouncedAsyncFileStore } from "./utils";

class SyncCombinedFileStore implements ISyncFileStore {
  asyncStore: AsyncFileStore;
  syncStore: MemoryFileStore;
  debouncedAsyncStore: IAsyncFileStore;
  constructor(asyncStore: AsyncFileStore, syncStore: MemoryFileStore) {
    this.asyncStore = asyncStore;
    this.syncStore = syncStore;
    this.debouncedAsyncStore = createDebouncedAsyncFileStore(asyncStore, 500);
    this.createFolder = this.createFolder.bind(this);
    this.deleteFolder = this.deleteFolder.bind(this);
    this.renameFolder = this.renameFolder.bind(this);
    this.getFileContent = this.getFileContent.bind(this);
    this.getAllFiles = this.getAllFiles.bind(this);
  }
  setFile(path: string, content: string): void {
    this.syncStore.setFile(path, content);
    this.debouncedAsyncStore.setFile(path, content);
  }
  ensureFile(path: string, defaultContent?: string | undefined): void {
    this.syncStore.ensureFile(path);
    this.asyncStore.ensureFile(path, defaultContent).catch(console.error);
  }
  getFilePaths(): string[] {
    return this.syncStore.getFilePaths();
  }
  addFile(path: string, content?: string | undefined): void {
    this.syncStore.addFile(path, content);
    this.debouncedAsyncStore.addFile(path, content);
  }
  addFiles(files: { path: string; content?: string | undefined; }[]): void {
    this.syncStore.addFiles(files);
    this.asyncStore.addFiles(files).catch(console.error);
  }
  renameFile(oldPath: string, newPath: string): void {
    this.syncStore.renameFile(oldPath, newPath);
    this.asyncStore.renameFile(oldPath, newPath).catch(console.error);
  }
  renameFiles(targets: { oldPath: string; newPath: string; }[]): void {
    this.syncStore.renameFiles(targets);
    this.asyncStore.renameFiles(targets).catch(console.error);
  }
  deleteFile(path: string): void {
    this.syncStore.deleteFile(path);
    this.asyncStore.deleteFile(path).catch(console.error);
  }
  deleteFiles(path: string[]): void {
    this.syncStore.deleteFiles(path);
    this.asyncStore.deleteFiles(path).catch(console.error);
  }
  createFolder(folder: string): void {
    this.syncStore.createFolder(folder);
    this.asyncStore.createFolder(folder).catch(console.error);
  }
  deleteFolder(folder: string): void {
    this.syncStore.deleteFolder(folder);
    this.asyncStore.deleteFolder(folder).catch(console.error);
  }
  renameFolder(oldPath: string, newPath: string): void {
    this.syncStore.renameFolder(oldPath, newPath);
    this.asyncStore.renameFolder(oldPath, newPath).catch(console.error);
  }
  getFileContent(path: string): string {
    return this.syncStore.getFileContent(path);
  }
  getAllFiles(): { path: string; content: string; }[] {
    return this.syncStore.getAllFiles();
  }
  refreshFromAsyncStore(): Promise<void> {
    return this.syncStore.loadFromAsyncStore(this.asyncStore);
  } 
  withEventSource(eventSource: string): SyncCombinedFileStore {
    return new SyncCombinedFileStore(this.asyncStore.withEventSource(eventSource), this.syncStore.withEventSource(eventSource));
  }

}

export {
  SyncCombinedFileStore,
}