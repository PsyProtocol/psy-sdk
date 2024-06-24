import { FileExplorer, FileExplorerConfig } from '@qstudio/file-explorer';
import styles from './FileExplorer.module.scss';
interface IFileExplorerDockComponentProps {
  config: FileExplorerConfig;
  onFileSelected: (file: string) => void;
}
const FileExplorerDockComponent: React.FC<IFileExplorerDockComponentProps> = ({config, onFileSelected}) => {
  return(
    <div className={styles.dockPage}>
      <FileExplorer config={config} onSelect={onFileSelected} />
    </div>
  )
};

export default FileExplorerDockComponent;