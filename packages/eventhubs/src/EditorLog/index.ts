enum EditorLogEventType {
  Clear = 0,
  Message = 1,
}

enum EditorLogMessageType {
  PlainText = 0,
  TextArea = 1,
}


enum EditorLogLevel {
  Error = 0,
  Warn = 1,
  Info = 2,
  Debug = 3,
  Trace = 4,
}

interface IEditorLogEventBase {
  type: EditorLogEventType;
}

// message types

interface IEditorLogEventClear extends IEditorLogEventBase {
  type: EditorLogEventType.Clear;
}

interface IEditorLogMessageEventBase extends IEditorLogEventBase {
  type: EditorLogEventType.Message;
  messageType: EditorLogMessageType;
}

// message events
interface IEditorLogPlainTextEvent extends IEditorLogMessageEventBase{
  messageType: EditorLogMessageType.PlainText;
  level: EditorLogLevel;
  message: string;
}

interface IEditorLogTextAreaEvent extends IEditorLogMessageEventBase{
  messageType: EditorLogMessageType.TextArea;
  level: EditorLogLevel;
  message: string;
}
type IEditorLogMessageEvent = IEditorLogPlainTextEvent | IEditorLogTextAreaEvent;

type IEditorLogEvent = IEditorLogEventClear | IEditorLogMessageEvent;

export {
  EditorLogEventType,
  EditorLogMessageType,
  EditorLogLevel,
}

export type {
  IEditorLogEventClear,
  IEditorLogEvent,

  IEditorLogMessageEvent,
  IEditorLogTextAreaEvent,
  IEditorLogPlainTextEvent,
}
