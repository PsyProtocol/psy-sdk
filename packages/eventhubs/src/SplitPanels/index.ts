enum SplitPanelsEventType {
  OpenTab = 'open-tab',
  OpenFileTab = 'open-file-tab',
  CloseTab = 'close-tab',
  CloseFileTab = 'close-file-tab',
  CloseAllFiles = 'close-all-files',
  ResizePanels = 'resize-panels',
}

interface ISplitPanelsEventBase {
  type: SplitPanelsEventType;
}


interface ISplitPanelsOpenTabEventBase extends ISplitPanelsEventBase {
  location?: string;
}

interface IOpenTabEvent extends ISplitPanelsOpenTabEventBase {
  type: SplitPanelsEventType.OpenTab;
  tabType: string;
}

interface IOpenFileTabEvent extends ISplitPanelsOpenTabEventBase {
  type: SplitPanelsEventType.OpenFileTab;
  filePath: string;
}

interface ICloseTabEvent extends ISplitPanelsEventBase {
  type: SplitPanelsEventType.CloseTab;
  tabType: string;
}

interface ICloseFileTabEvent extends ISplitPanelsEventBase {
  type: SplitPanelsEventType.CloseFileTab;
  filePath: string;
}

interface ICloseAllFilesEvent extends ISplitPanelsEventBase {
  type: SplitPanelsEventType.CloseAllFiles;
}

interface IResizePanelsEvent extends ISplitPanelsEventBase {
  type: SplitPanelsEventType.ResizePanels;
}

type SplitPanelsEvent = IOpenTabEvent | IOpenFileTabEvent | ICloseTabEvent | ICloseFileTabEvent | ICloseAllFilesEvent | IResizePanelsEvent;

export {
  SplitPanelsEventType,
}

export type {
  SplitPanelsEvent,
  IOpenTabEvent,
  IOpenFileTabEvent,
  ICloseTabEvent,
  ICloseFileTabEvent,
  ICloseAllFilesEvent,
}
