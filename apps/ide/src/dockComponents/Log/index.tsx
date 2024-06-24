import { EventHub } from '@qstudio/utils';
import { EditorLog } from '../../components/Log';
import styles from './Log.module.scss';
import { EditorLogEventType, IEditorLogEvent } from '@qstudio/eventhubs';
import { IDEContext } from '../../utils/ideContext';

interface ILogDockComponentProps {
  ideContext: IDEContext;
  className?: string;
}
const LogDockComponent: React.FC<ILogDockComponentProps> = ({ideContext}) => {
  return(
    <div className={styles.logDockComponent}>
      <EditorLog eventHub={ideContext.logEventHub} className={styles.editorLog} initialMessages={[...ideContext.logMessages]}/>
    </div>
  )
};

export default LogDockComponent;