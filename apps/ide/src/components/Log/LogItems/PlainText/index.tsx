import { EditorLogLevel, IEditorLogPlainTextEvent } from '@qstudio/eventhubs';
import styles from './PlainText.module.scss';
import classNames from 'classnames';

interface IPlainTextProps extends IEditorLogPlainTextEvent {
  className?: string;
}

const logLevelClassName : Record<EditorLogLevel, string> = {
  [EditorLogLevel.Error]: styles.error,
  [EditorLogLevel.Warn]: styles.warn,
  [EditorLogLevel.Info]: styles.info,
  [EditorLogLevel.Trace]: styles.trace,
  [EditorLogLevel.Debug]: styles.debug,
};


const PlainText: React.FC<IPlainTextProps> = ({level, message, className}) => {
  return (
    <span className={classNames(styles.plainText, logLevelClassName[level], className)}>{message}</span>
  );
}

export {
  PlainText,
};