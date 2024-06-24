enum EditorUIEventType {
  CommandBar = 0,
  OpenProject = 1,
  CloseCommandBar = 2,
}


enum IDEMenuId {
  // 0-256 are reserved for root menus and core sub menus
  Standard = 0,
  Files = 1,
  Projects = 2,
  NewProject = 3,
  RenameProject = 4,

  //256 - 512 are reserved for file actions
}



interface IEditorUIEventBase {
  type: EditorUIEventType;
}

// message types

interface IEditorUICommandBarEvent extends IEditorUIEventBase {
  type: EditorUIEventType.CommandBar;
  menuType: IDEMenuId;
  defaultValue?: string;
  originId?: string;
}


interface IEditorUICloseCommandBarEvent extends IEditorUIEventBase {
  type: EditorUIEventType.CloseCommandBar;
  originId?: string;
}

interface IEditorUIOpenProjectEvent extends IEditorUIEventBase {
  type: EditorUIEventType.OpenProject;
  projectId: string;
}



type IEditorUIEvent = IEditorUICommandBarEvent | IEditorUIOpenProjectEvent | IEditorUICloseCommandBarEvent;

export {
  EditorUIEventType,
  IDEMenuId,
}

export type {
  IEditorUICommandBarEvent,
  IEditorUIOpenProjectEvent,
  IEditorUICloseCommandBarEvent,
  
  IEditorUIEvent,
}
