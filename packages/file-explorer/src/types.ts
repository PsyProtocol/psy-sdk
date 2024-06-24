import { ProjectFilesEvent, ProjectFilesEventType } from "@qstudio/eventhubs";
import { ISyncFileStore } from "@qstudio/storage";
import { EventHub } from "@qstudio/utils";


interface ISimpleFilePathStore {
  getFilePaths(): Promise<string[]>;
  addFilePath(filePath: string): Promise<void>;
  renameFilePath(oldPath: string, newPath: string): Promise<void>;
  deleteFilePath(filePath: string): Promise<void>;
  createFolder(folderPath: string): Promise<void>;
  deleteFolder(folderPath: string): Promise<void>;
  renameFolder(oldPath: string, newPath: string): Promise<void>;
}
interface IFileExplorerConfigProps {
  fileIcons?: Record<string, IFileIconDef>;
  defaultFileIcon?: IFileIconDef;
  defaultFolderIcon?: IFileIconDef;
  fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>;
  store: ISyncFileStore;
}

interface FileIconBaseProps extends React.SVGAttributes<SVGElement> {
  children?: React.ReactNode;
  size?: string | number;
  color?: string;
  title?: string;
}
type FileIconType = (props: FileIconBaseProps) => JSX.Element;
interface IFileIconDef {
  icon?: FileIconType;
  iconColor?: string;
}

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
  IFileExplorerConfigProps,
  IFileIconDef,
  ISimpleFilePathStore,
}