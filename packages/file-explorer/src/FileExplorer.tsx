import React from 'react';
import styles from './FileExplorer.module.scss';
// 1: Uncontrolled Tree
import { useEffect, useRef, useState } from "react";

import { Tree, useSimpleTree, TreeApi, NodeApi, RowRendererProps } from "react-arborist";

import Node from "./Node";

import { TbFolderPlus } from "react-icons/tb";
import { AiOutlineFileAdd } from "react-icons/ai";
import useResizeObserver from "use-resize-observer";
import {
  Menu,
  Item,
  Separator,
  Submenu,
  useContextMenu
} from "react-contexify";

import { useSimpleTreeV3 } from './hooks/useSimpleTreeV3';
import { FileExplorerConfig } from './FileExplorerConfig';
import { IFileNodeData } from './types';


interface IFileExplorerProps {
  onSelect: (v: string) => any;
  config: FileExplorerConfig;
}
function getFullPathParts(x: NodeApi<IFileNodeData> | null) {
  if (!x || x.level === -1) {
    return [];
  }
  let cur: NodeApi<IFileNodeData> | null = x;
  const parts: string[] = [];
  while (cur && cur.level !== -1) {
    parts.push(cur.data.name);
    cur = cur.parent;
  }
  return parts.reverse();
}
function moveFile(renameFile: (fileName: string, newFileName: string) => any, node: NodeApi<IFileNodeData>, oldParent: NodeApi<IFileNodeData> | null, newParent: NodeApi<IFileNodeData> | null) {
  const oldParentPath = getFullPathParts(oldParent).join("/");
  const newParentPath = getFullPathParts(newParent).join("/");
  if (oldParentPath !== newParentPath) {
    const oldPath = oldParentPath ? (oldParentPath + "/" + node.data.name) : node.data.name;
    const newPath = newParentPath ? (newParentPath + "/" + node.data.name) : node.data.name;
    renameFile(oldPath, newPath);
  }
}
const MENU_ID = "fileTreeMenu";


function handleRenameClick({ event, props, triggerEvent, data }: any) {

  return props.startRename();
}

function handleDeleteClick({ event, props, triggerEvent, data }: any) {
}
function handleCreate({ event, props, triggerEvent, data }: any) {

  return props.startCreate();
}
function handleCreateFolder({ event, props, triggerEvent, data }: any) {
  return props.startCreateFolder();
}
const FileExplorer: React.FC<IFileExplorerProps> = ({ config, onSelect }: IFileExplorerProps) => {
  const [term, setTerm] = useState("");
  const treeRef = useRef<TreeApi<any>>(null);
  const { ref, width, height } = useResizeObserver<HTMLDivElement>();


  const { show } = useContextMenu({
    id: MENU_ID
  });
  const [data, { onCreate, onDelete, onMove, onRename, resetData }, store] = useSimpleTreeV3(config);

  const createFileFolder = (
    <>
      <input
        type="text"
        placeholder="Search..."
        className={styles.searchInput}
        name="search"
        value={term}
        onChange={(e) => setTerm(e.target.value)}
      />
      <button
        onClick={() => treeRef.current?.createInternal()}
        title="New Folder..."
      >
        <TbFolderPlus />
      </button>
      <button onClick={() => treeRef.current?.createLeaf()} title="New File...">
        <AiOutlineFileAdd />
      </button>
    </>
  );
  function DefaultRow({
    node,
    attrs,
    innerRef,
    children,
  }: RowRendererProps<IFileNodeData>) {
    return (
      <div
        {...attrs}
        ref={innerRef}
        onFocus={(e) => e.stopPropagation()}
        onClick={node.handleClick}
        onContextMenu={(e) => {
          //          const value = (node.data as any).name;

          show({
            event: e,
            props: {
              id: node.id, startRename: () => {
                const runRename: any = (r: any) => {
                  if (r && !r.cancelled) {
                    if (!r.value) {
                      return treeRef.current?.edit(node.id).then(runRename);
                    } else {
                      const oldPathParts = getFullPathParts(node);
                      const oldFullPath = oldPathParts.join("/");
                      oldPathParts.pop();
                      oldPathParts.push(r.value);
                      const newFullPath = oldPathParts.join("/");
                      store.renameFile(oldFullPath, newFullPath);
                      //renameFile(oldFullPath, newFullPath);
                      return Promise.resolve(r);

                    }
                  } else {
                    return Promise.resolve(r);
                  }

                };
                return treeRef.current?.edit(node.id).then(runRename).catch(console.error);
              }, startCreate: () => {
                if (node.isInternal) {
                  treeRef.current?.create({
                    parentId: node.id,
                    type: "leaf",
                  });
                } else {
                  if (node.parent) {
                    treeRef.current?.create({
                      parentId: node.parent.id,
                      type: "leaf",
                    });
                  } else {
                    treeRef.current?.create({
                      parentId: null,
                      type: "leaf",
                    });
                  }
                }
              }, startCreateFolder: () => {
                if (node.isInternal) {
                  treeRef.current?.create({
                    parentId: node.id,
                    type: "internal",
                  });
                } else {
                  if (node.parent) {
                    treeRef.current?.create({
                      parentId: node.parent.id,
                      type: "internal",
                    });
                  } else {
                    treeRef.current?.create({
                      parentId: null,
                      type: "internal",
                    });
                  }
                }
              }
            },
          });
        }}
      >
        {children}
      </div>
    );
  }
  return (
    <div className={styles.fileTreeCon}>
      <div className={styles.folderFileActions}>{createFileFolder}</div>
      <div className={styles.treeCon} ref={ref}>
      <Tree<any>
        ref={treeRef}
        data={data}
        indent={16}
        rowHeight={24}
        width={width}
        height={(height||100)-1}

        // openByDefault={false}
        searchTerm={term}
        disableMultiSelection={true}

        renderRow={DefaultRow}
        onRename={(a) => {
          const isNewlyCreated = !!((!a.node.data.name) && a.name);
          const oldPathParts = getFullPathParts(a.node);
          const oldFullPath = oldPathParts.join("/");
          const oldName = oldPathParts.pop();
          if (!oldName && !a.name) {
            treeRef.current?.delete(a.node.id);
            return;
          } else if (!a.name) {
            a.name = oldName as any;

          }

          oldPathParts.push(a.name);

          const newFullPath = oldPathParts.join("/");
          if(isNewlyCreated){
            store.renameFile(oldFullPath, newFullPath);
            store.ensureFile(newFullPath, "");
          }else{
            store.renameFile(oldFullPath, newFullPath);
          }
          return onRename(a);
        }}
        onCreate={(d) => {

          const result: any = onCreate(d);
          if (result && result.id) {
            setTimeout(() => {
              if (treeRef.current) {
                const r = treeRef.current.get(result.id);
                if (r) {
                  const parts = getFullPathParts(r);
                  if (parts[parts.length - 1].indexOf(".") !== -1) {

                    store.addFile(parts.join("/"), "");
                  }
                }
              }
            }, 100);
          }
          return result;
        }}
        onMove={(a) => {
          const newParent = a.parentNode;
          if (a.dragNodes.filter(x => x.isInternal).length !== 0) {
            return;
          }
          a.dragNodes.forEach(n => {
            const oldParent = n.parent;
            moveFile((fileName: string, newFileName: string) => {
              store.renameFile(fileName, newFileName);
            }, n, oldParent, newParent);
          });
          return onMove(a);
        }}
        onDelete={(a) => {
          const remFiles = a.nodes.map(n => {
            return (getFullPathParts(n).join("/"));
          });
          remFiles.forEach((f) => store.deleteFile(f));

          onDelete(a);
        }}
        onSelect={(nodes) => {

          if (nodes[0]) {
            const dz = getFullPathParts(nodes[0]);
            if (dz[dz.length - 1].indexOf(".") !== -1) {
              
              onSelect(dz.join("/"));
            }
          }
        }}
        searchMatch={(node, term) =>
          node.data.name.toLowerCase().includes(term.toLowerCase())
        }
      >
        {Node}
      </Tree>


      <Menu id={MENU_ID}>
        <Item onClick={handleCreate}>
          New File...
        </Item>
        <Item onClick={handleCreateFolder}>
          New Folder..
        </Item>
        <Separator />
        <Item onClick={handleRenameClick}>
          Rename
        </Item>
        <Item onClick={handleDeleteClick}>
          Delete
        </Item>
      </Menu>
    </div>
    </div>
  );
};

export default FileExplorer;
