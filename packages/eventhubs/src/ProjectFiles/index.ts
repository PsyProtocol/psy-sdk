enum ProjectFilesEventType {
  FileCreated = 'file-created',
  FileDeleted = 'file-deleted',
  FileModified = 'file-modified',
  FileRenamed = 'file-renamed',
  FolderCreated = 'folder-created',
  FolderDeleted = 'folder-deleted',
  FolderRenamed = 'folder-modified',
  RefreshAll = 'refresh-all',
}

interface IProjectFilesEventBase {
  type: ProjectFilesEventType;
  path: string;
  eventSource?: string;
}

interface IFileCreatedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FileCreated;
}
interface IRefreshAllEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.RefreshAll;
}

interface IFileDeletedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FileDeleted;
}

interface IFileModifiedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FileModified;
}

interface IFileRenamedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FileRenamed;
  newPath: string;
}

interface IFolderCreatedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FolderCreated;
}

interface IFolderDeletedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FolderDeleted;
}

interface IFolderRenamedEvent extends IProjectFilesEventBase {
  type: ProjectFilesEventType.FolderRenamed;
  newPath: string;
}

type ProjectFilesEvent = IFileCreatedEvent | IFileDeletedEvent | IFileModifiedEvent | IFileRenamedEvent | IFolderCreatedEvent | IFolderDeletedEvent | IFolderRenamedEvent | IRefreshAllEvent;

export {
  ProjectFilesEventType,
}

export type {
  ProjectFilesEvent,
  IFileCreatedEvent,
  IFileDeletedEvent,
  IFileModifiedEvent,
  IFileRenamedEvent,
  IFolderCreatedEvent,
  IFolderDeletedEvent,
  IFolderRenamedEvent,
  IRefreshAllEvent,
}