import {useState, useEffect, useRef} from 'react';
import styles from './EditorLog.module.scss';
import classNames from 'classnames';
import { EditorLogEventType, EditorLogMessageType, IEditorLogEvent, IEditorLogEventClear, IEditorLogMessageEvent } from "@qstudio/eventhubs";
import { EventHub } from "@qstudio/utils";
import { PlainText } from './LogItems/PlainText';

interface ILogProps {
  eventHub: EventHub<EditorLogEventType, IEditorLogEvent>;
  initialMessages?: IEditorLogMessageEvent[];
  className?: string;
}
type TLogComponentMap = {
  [E in IEditorLogMessageEvent as E["messageType"]]: React.FC<E>;
}


const EDITOR_COMPONENTS: TLogComponentMap = {
  [EditorLogMessageType.PlainText]: PlainText,
  [EditorLogMessageType.TextArea]: (ev)=><div><textarea readOnly value={ev.message} /></div>,
};

const LogItem: React.FC<IEditorLogMessageEvent> = (props)=>{
  const LogComponent = EDITOR_COMPONENTS[props.messageType];
  return <LogComponent {...(props as any)} />;
};

const EditorLog: React.FC<ILogProps> = ({eventHub, initialMessages, className})=>{
  const scrollDivRef = useRef<HTMLDivElement>(null);
  const [logItems, setLogItems] = useState<IEditorLogMessageEvent[]>(initialMessages||[]);

  useEffect(()=>{
    const onMessage = (event: IEditorLogMessageEvent)=>{
      setLogItems((prev)=>[...prev, event]);
    };

    const onClear = (_: IEditorLogEventClear)=>{
      setLogItems([]);
    };

    eventHub.on(EditorLogEventType.Clear, onClear);
    eventHub.on(EditorLogEventType.Message, onMessage);

    return ()=>{
      eventHub.remove(EditorLogEventType.Message, onMessage);
      eventHub.remove(EditorLogEventType.Clear, onClear);
    };
  },[eventHub]);

  useEffect(()=>{
    if(scrollDivRef.current){
      scrollDivRef.current.scrollTo({left: 0, top: scrollDivRef.current.scrollHeight, behavior: "smooth"});
    }
  },[logItems])

  return(
    <div className={classNames(styles.editorLogContainer, className)} ref={scrollDivRef}>
      {logItems.map((item, i)=><LogItem key={i} {...item} />)}
    </div>
  );
}

export {
  EditorLog,
}