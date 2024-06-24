interface IAsyncFileStore {
  getFilePaths(): Promise<string[]>;
  ensureFile(path: string, defaultContent?: string): Promise<void>;
  addFile(path: string, content?: string): Promise<void>;
  setFile(path: string, content: string): Promise<void>;
  addFiles(files: {path: string, content?: string}[]): Promise<void>;
  renameFile(oldPath: string, newPath: string): Promise<void>;
  renameFiles(targets: {oldPath: string, newPath: string}[]): Promise<void>;
  deleteFile(path: string): Promise<void>;
  deleteFiles(path: string[]): Promise<void>;
  createFolder(folder: string): Promise<void>;
  deleteFolder(folder: string): Promise<void>;
  renameFolder(oldPath: string, newPath: string): Promise<void>;
  getFileContent(path: string): Promise<string>;
  getAllFiles(): Promise<{path: string, content: string}[]>;
}
interface ISyncFileStore {
  getFilePaths(): string[];
  ensureFile(path: string, defaultContent?: string): void;
  addFile(path: string, content?: string): void;
  setFile(path: string, content: string): void;
  addFiles(files: {path: string, content?: string}[]): void;
  renameFile(oldPath: string, newPath: string): void;
  renameFiles(targets: {oldPath: string, newPath: string}[]): void;
  deleteFile(path: string): void;
  deleteFiles(path: string[]): void;
  createFolder(folder: string): void;
  deleteFolder(folder: string): void;
  renameFolder(oldPath: string, newPath: string): void;
  getFileContent(path: string): string;
  getAllFiles(): {path: string, content: string}[];
  withEventSource(eventSource: string): ISyncFileStore;
}
interface IAsyncGlobalKVStore {
  getItem<T>(key: string): Promise<T | null>;
  setItem<T>(key: string, value: T): Promise<void>;
  removeItem(key: string): Promise<void>;
  addToSet<T>(key: string, item: T, compare: (a: T, b: T) => boolean, replace?: boolean): Promise<T[]>;
  removeFromSet<T>(key: string, item: T, compare: (a: T, b: T) => boolean): Promise<T[]>;
}
export type {
  IAsyncFileStore,
  ISyncFileStore,
  IAsyncGlobalKVStore,
};