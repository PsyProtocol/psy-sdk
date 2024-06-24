
import { SlDoc } from "react-icons/sl";

import { SiHtml5, SiJavascript, SiCss3,SiTypescript, SiBitcoin, SiAssemblyscript, SiMarkdown } from "react-icons/si";
import {v4 as uuidv4} from 'uuid';
const FILE_EXTENSION_LANGUAGE_MAP: Record<string, string> = {
  "ts": "typescript",
  "tsx": "typescript",
  "js": "javascript",
  "dapen": "dapen",
  "dpn": "dapen",
  "json": "json",
  "py": "python",
  "html": "html",
  "css": "css",
  "scss": "scss",
  "wgsl": "wgsl",
}

function getLanguageForFilePath(filePath: string){
  const ext = getFileExtForFilePath(filePath);
  const result = FILE_EXTENSION_LANGUAGE_MAP[ext];
  if(result && typeof result === 'string'){
    return result;
  }else{
    return "plaintext";
  }
}
function getFileExtForFileName(fileName: string){
  const dotInd = fileName.lastIndexOf(".");
  if(dotInd===-1){
    return "";
  }else{
    return fileName.substring(dotInd+1);
  }
}
function getFileNameForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  return filePath.substring(slashInd+1);
}
function getFileFolderForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  return filePath.substring(0, slashInd);
}
function getFileExtForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  return getFileExtForFileName(filePath.substring(slashInd+1));
}

function getFileDisplayForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  const fileName = slashInd === -1 ? filePath : filePath.substring(slashInd+1);
  const parentFolder = slashInd === -1 ? "" : filePath.substring(0, slashInd);
  const dotInd = filePath.lastIndexOf(".");
  const ext = dotInd === -1 ? "" : filePath.substring(dotInd+1);
  const nodeData = getFileNodeDataForFileName(fileName);
  return {
    name: fileName,
    ext: ext,
    icon: nodeData.icon,
    iconColor: nodeData.iconColor,
    id: nodeData.id,
    parentFolder: parentFolder,
  }
}


function getFileDisplayForFileName(fileName: string){
  const nodeData = getFileNodeDataForFileName(fileName);
  return {
    name: fileName,
    ext: getFileExtForFileName(fileName),
    icon: nodeData.icon,
    iconColor: nodeData.iconColor,
    id: nodeData.id,
  }
}
function getFileNodeDataForFileName(fileName: string): IFileNodeData{
  const ext = getFileExtForFileName(fileName);
  if(ext === "js"){
    return {
      icon: SiJavascript,
      iconColor: "#efd81e",
      id: uuidv4(),
      name: fileName,
    }
  }else if(ext === "ts"){
    return {
      icon: SiTypescript,
      iconColor: "#007acc",
      id: uuidv4(),
      name: fileName,
    }
  }else if(ext === "basm"){
    return {
      icon: SiBitcoin,
      iconColor: "#007acc",
      id: uuidv4(),
      name: fileName,
    }
  }else{
    return {
      icon: SlDoc,
      iconColor: "#f0f0f0",
      id: uuidv4(),
      name: fileName,
    }
  }
}
function ensureFile(parent: IFileNodeData, path: string[], pathIndex = 0){
  const pathPart = path[pathIndex];
  if(pathIndex === (path.length-1)){
    if(parent.children){
      if(!parent.children.find(x=>x.name === pathPart)){
        parent.children.push(getFileNodeDataForFileName(pathPart));
      }
    }else{
      parent.children=[getFileNodeDataForFileName(pathPart)];
    }
  }else{
    let child: IFileNodeData = {
      icon: undefined,
      id: uuidv4(),
      name: pathPart,
      children: []
    };
    if(parent.children){
      const tChild = parent.children.find(x=>x.name === pathPart);
      if(!tChild){
        parent.children.push(child);
      }else{
        child = tChild;
      }
    }else{
      parent.children = [child]
    }
    ensureFile(child, path, pathIndex+1);
  }
}
function filePathsToTree(filePaths: string[]){

  const root : IFileNodeData = {
    name: "root",
    id:"1337",
    children: [],
  };
  for(const filePath of filePaths){
    ensureFile(root, filePath.split("/"), 0);
  }
  return (root.children||[]);
}



interface FileIconBaseProps extends React.SVGAttributes<SVGElement> {
  children?: React.ReactNode;
  size?: string | number;
  color?: string;
  title?: string;
}
type FileIconType = (props: FileIconBaseProps) => JSX.Element;


interface IFileNodeData {
  icon?: any;
  iconColor?: string;
  name: string;
  id: string;
  children?: IFileNodeData[];
}

export type {
  FileIconBaseProps,
  FileIconType,
  IFileNodeData,
}
export {
  filePathsToTree,
  getFileExtForFileName,
  getFileExtForFilePath,
  getFileNodeDataForFileName,
  getFileNameForFilePath,
  getFileDisplayForFileName,
  getFileDisplayForFilePath,
  getFileFolderForFilePath,
  getLanguageForFilePath,
}