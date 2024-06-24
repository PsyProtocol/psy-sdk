import { useEffect, useMemo, useState } from "react";
import { SimpleTree, MoveHandler, RenameHandler, CreateHandler, DeleteHandler } from "react-arborist";
import { IFileNodeData } from "../types";
import { FileExplorerConfig } from "../FileExplorerConfig";
import { uuidv4 } from "@qstudio/utils";
import { ProjectFilesEvent, ProjectFilesEventType } from "@qstudio/eventhubs";

export type SimpleTreeData = {
    id: string;
    name: string;
    children?: SimpleTreeData[];
};

let nextId = 0;

function useSimpleTreeV3(config: FileExplorerConfig) {
    const [eventSourceId, setEventSourceId] = useState<string>(()=>uuidv4());
    const [data, setData] = useState<IFileNodeData[]>(()=>config.getAllFileNodeData());
    const store = useMemo(()=>config.store.withEventSource(eventSourceId), [config, eventSourceId]);
    const tree = useMemo(
        () =>
            new SimpleTree<// @ts-ignore
                T>(data),
        [data,config]
    );

    const onMove: MoveHandler<IFileNodeData> = (args: {
        dragIds: string[];
        parentId: null | string;
        index: number;
    }) => {
        for (const id of args.dragIds) {
            tree.move({ id, parentId: args.parentId, index: args.index });
        }
        setData(tree.data);
    };

    const onRename: RenameHandler<IFileNodeData> = ({ name, id }) => {
        if(name.indexOf(".") !== -1){
            const nodeData = config.getFileNodeDataForFileName(name);
            tree.update({ id, changes: { name,icon:nodeData.icon, iconColor: nodeData.iconColor } as any });
            setData(tree.data);
        }else{
            tree.update({ id, changes: { name } as any });
            setData(tree.data);
        }
    };

    const onCreate: CreateHandler<IFileNodeData> = ({ parentId, index, type }) => {
        const data = { id: `simple-tree-id-${nextId++}`, name: "" } as any;
        if (type === "internal") data.children = [];
        tree.create({ parentId, index, data });
        setData(tree.data);
        return data;
    };

    const onDelete: DeleteHandler<IFileNodeData> = (args: { ids: string[] }) => {
        args.ids.forEach((id) => tree.drop({ id }));
        setData(tree.data);
    };

    const resetData: (newData: IFileNodeData[]) => void = (newData: IFileNodeData[]) => {
        setData(newData);
    };
    const controller = { onMove, onRename, onCreate, onDelete, resetData };

    useEffect(()=>{
      const onChanged = (event: ProjectFilesEvent)=>{
        if(event.eventSource === eventSourceId){
          return;
        }else{
          setData(config.getAllFileNodeData());
        }
      }
      const eventTypes = [ProjectFilesEventType.FileCreated, ProjectFilesEventType.FileDeleted, ProjectFilesEventType.FileRenamed, ProjectFilesEventType.FolderCreated, ProjectFilesEventType.FolderDeleted, ProjectFilesEventType.FolderRenamed, ProjectFilesEventType.RefreshAll];
      config.fileEventHub.onOneOf(eventTypes, onChanged);
      return ()=>{
        config.fileEventHub.removeOneOf(eventTypes, onChanged);
      };
    },[eventSourceId,config]);

    useEffect(()=>{
        setData(config.getAllFileNodeData());
    },[config]);

    return [data, controller, store] as const;
}


export {
    useSimpleTreeV3,
}