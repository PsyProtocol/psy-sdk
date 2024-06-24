import { debouncePromise, } from "@qstudio/utils";
import { IAsyncFileStore } from "./types";
function filePathInFolder(filePath: string, folderPath: string) {
  return filePath.startsWith(folderPath);
}
function renameFolderInPath(filePath: string, oldPath: string, newPath: string) {
  if (filePath.startsWith(oldPath)) {
    return newPath + filePath.slice(oldPath.length);
  }
  return filePath;
}
function getFoldersAndFiles(filePaths: string[]){

  const folderMap: Record<string, boolean> = {};
  filePaths.forEach(fp=>{
    const folders = fp.split("/").filter(x=>x);
    folders.pop();
    while(folders.length){
      const folder = folders.join("/");
      folderMap[folder] = true;
      folders.pop();
    }
  });
  const folders = Object.keys(folderMap);
  const files = filePaths.filter(x=>!Object.hasOwnProperty.call(folderMap, x) && !folderMap[x]);
  return {
    folders,
    files,
  }

}
function createDebouncedAsyncFileStore(store: IAsyncFileStore, wait?: number, options?: any): IAsyncFileStore {
  return {
    addFile: debouncePromise(store.addFile.bind(store), wait, options),
    addFiles: debouncePromise(store.addFiles.bind(store), wait, options),
    createFolder: debouncePromise(store.createFolder.bind(store), wait, options),
    deleteFile: debouncePromise(store.deleteFile.bind(store), wait, options),
    deleteFiles: debouncePromise(store.deleteFiles.bind(store), wait, options),
    deleteFolder: debouncePromise(store.deleteFolder.bind(store), wait, options),
    getFileContent: debouncePromise(store.getFileContent.bind(store), wait, options),
    getAllFiles: debouncePromise(store.getAllFiles.bind(store), wait, options),
    getFilePaths: debouncePromise(store.getFilePaths.bind(store), wait, options),
    renameFile: debouncePromise(store.renameFile.bind(store), wait, options),
    renameFiles: debouncePromise(store.renameFiles.bind(store), wait, options),
    renameFolder: debouncePromise(store.renameFolder.bind(store), wait, options),
    setFile: debouncePromise(store.setFile.bind(store), wait, options),
    ensureFile: debouncePromise(store.ensureFile.bind(store), wait, options),
  };
}
export {
  filePathInFolder,
  renameFolderInPath,
  createDebouncedAsyncFileStore,
  getFoldersAndFiles,
}