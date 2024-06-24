import { ISyncFileStore } from "@qstudio/storage";

function loadProjectIfEmpty(fs: ISyncFileStore, projectFiles: Record<string, string>){
  const files = fs.getFilePaths();
  if(files.length === 0){
    fs.addFiles(Object.keys(projectFiles).map((path)=>({path, content: projectFiles[path]})));
    return true;
  }
  return false;
}

export {
  loadProjectIfEmpty,
}