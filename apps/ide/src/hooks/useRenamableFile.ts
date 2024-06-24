import { useState, useEffect } from 'react';
import { EventHub } from '@qstudio/utils';
import { ProjectFilesEvent, ProjectFilesEventType, IFileRenamedEvent } from '@qstudio/eventhubs';

export function useRenamableFile(initialFileName: string, fileEventHub: EventHub<ProjectFilesEventType, ProjectFilesEvent>) {
  const [fileName, setFileName] = useState(initialFileName);
  useEffect(() => {
    let curFileName = fileName;
    function handleRename(ev: IFileRenamedEvent) {
      if(curFileName === ev.path) {
        curFileName = ev.newPath;
        setFileName(ev.newPath);
      }
    }
    fileEventHub.on(ProjectFilesEventType.FileRenamed, handleRename);
    return () => {
      fileEventHub.removeEventListener(ProjectFilesEventType.FileRenamed, handleRename);
    };
  }, [fileEventHub, fileName]);
  return fileName;
}
