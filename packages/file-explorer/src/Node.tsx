import React from "react";
import { AiFillFolder, AiFillFile } from "react-icons/ai";
import { MdArrowRight, MdArrowDropDown, MdEdit } from "react-icons/md";
import { RxCross2 } from "react-icons/rx";

import styles from './Node.module.scss';

const Node = ({ node, style, dragHandle, tree }: any) => {
  const CustomIcon = node.data.icon;
  const iconColor = node.data.iconColor;

  return (
    <div
      className={`${styles.nodeContainer} ${node.state.isSelected ? "isSelected" : ""}`}
      style={style}
      ref={dragHandle}
    >
      <div
        className={styles.nodeContent}
        onClick={() => node.isInternal && node.toggle()}
      >
        {node.isLeaf ? (
          <>
            <span className={styles.arrow}></span>
            <span className={styles.fileFolderIcon}>
              {CustomIcon ? (
                <CustomIcon color={iconColor ? iconColor : "#6bc7f6"} />
              ) : (
                <AiFillFile color="#6bc7f6" />
              )}
            </span>
          </>
        ) : (
          <>
            <span className={styles.arrow}>
              {node.isOpen ? <MdArrowDropDown /> : <MdArrowRight />}
            </span>
            <span className={styles.fileFolderIcon}>
              {CustomIcon ? (
                <CustomIcon color={iconColor ? iconColor : "#f6cf60"} />
              ) : (
                <AiFillFolder color="#f6cf60" />
              )}
            </span>
          </>
        )}
        <span className={styles.nodeText}>
          {node.isEditing ? (
            <input
              type="text"
              defaultValue={node.data.name}
              onFocus={(e) => e.currentTarget.select()}
              onBlur={() => node.reset()}
              onKeyDown={(e) => {
                if (e.key === "Escape") node.reset();
                if (e.key === "Enter") node.submit(e.currentTarget.value);
              }}
              autoFocus
            />
          ) : (
            <span>{node.data.name}</span>
          )}
        </span>
      </div>

      <div className={styles.fileActions}>
        <div className={styles.folderFileActions}>
          <button onClick={() => node.edit()} title="Rename...">
            <MdEdit />
          </button>
          <button onClick={() => tree.delete(node.id)} title="Delete">
            <RxCross2 />
          </button>
        </div>
      </div>
    </div>
  );
};

export default Node;
