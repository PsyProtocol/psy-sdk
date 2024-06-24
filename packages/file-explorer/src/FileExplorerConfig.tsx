import React from "react";
import {ProjectFilesEventType, ProjectFilesEvent }from '@qstudio/eventhubs';
import type { IFileExplorerConfigProps, IFileIconDef, IFileNodeData } from "./types";
import { EventHub, uuidv4 } from "@qstudio/utils";
import { getFileExtForFileName } from "./utils/fileNodeData";
import {ISyncFileStore} from '@qstudio/storage';

class FileExplorerConfig {
  fileIcons: Record<string, IFileIconDef> = {};
  defaultFileIcon: IFileIconDef = { icon: () => <span /> };
  defaultFolderIcon: IFileIconDef = { icon: () => <span /> };
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  store: ISyncFileStore;
  
  constructor(props: IFileExplorerConfigProps) {
    this.fileIcons = props.fileIcons || {};
    this.defaultFileIcon = props.defaultFileIcon || this.defaultFileIcon;
    this.defaultFolderIcon = props.defaultFolderIcon || this.defaultFolderIcon;
    this.fileEventHub = props.fileEventHub;
    this.store = props.store;
  }

  getFileNodeDataForFileName(fileName: string): IFileNodeData {
    const ext = getFileExtForFileName(fileName);
    if (Object.hasOwnProperty.call(this.fileIcons, ext)) {
      return {
        icon: this.fileIcons[ext].icon,
        iconColor: this.fileIcons[ext].iconColor,
        id: uuidv4(),
        name: fileName,
      }
    } else {
      return {
        icon: this.defaultFileIcon.icon,
        iconColor: this.defaultFileIcon.iconColor,
        id: uuidv4(),
        name: fileName,
      }
    }
  }
  ensureFile(parent: IFileNodeData, path: string[], pathIndex = 0) {
    const pathPart = path[pathIndex];
    if (pathIndex === (path.length - 1)) {
      if (parent.children) {
        if (!parent.children.find((x) => x.name === pathPart)) {
          parent.children.push(this.getFileNodeDataForFileName(pathPart));
        }
      } else {
        parent.children = [this.getFileNodeDataForFileName(pathPart)];
      }
    } else {
      let child: IFileNodeData = {
        icon: undefined,
        id: uuidv4(),
        name: pathPart,
        children: []
      };
      if (parent.children) {
        const tChild = parent.children.find(x => x.name === pathPart);
        if (!tChild) {
          parent.children.push(child);
        } else {
          child = tChild;
        }
      } else {
        parent.children = [child]
      }
      this.ensureFile(child, path, pathIndex + 1);
    }
  }
  filePathsToTree(filePaths: string[]): IFileNodeData[] {

    const root: IFileNodeData = {
      name: "root",
      id: "1337",
      children: [],
    };
    for (const filePath of filePaths) {
      this.ensureFile(root, filePath.split("/"), 0);
    }
    return (root.children || []);
  }
  getAllFilePaths(){
    return this.store.getFilePaths();
  }
  getAllFileNodeData(): IFileNodeData[] {
    return this.filePathsToTree(this.getAllFilePaths());
  }
  getAllStandardFiles(){
    return this.getAllFilePaths().filter(x=>x.indexOf(".")!==-1);
  }

}

export {
  FileExplorerConfig,
}