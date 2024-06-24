import { RectSide } from "../../anchor";
import { QWRect } from "./QWRect";
import { QWTreeJunction } from "./QWTreeJunction";
import { QWidget } from "./QWidget";

interface ITreeNodeID {
  level: number;
  index: number;
  isLeaf: boolean;
  isRoot: boolean;
}
function buildSimpleTree(
  treeHeight: number,
  nodeGen: (
    srp: ITreeNodeID,
    children: QWidget<any, any, any>[]
  ) => QWidget<any, any, any>
): QWidget<any, any, any> {
  if (treeHeight < 0) {
    throw new Error("Tree height must be greater than or equal to 0");
  }
  if (treeHeight === 0) {
    return nodeGen({ level: 0, index: 0, isLeaf: true, isRoot: true }, []);
  }

  let level = treeHeight;
  const numLeaves = Math.pow(2, treeHeight);
  let currentLevelNodes: QWidget<any, any, any>[] = [];
  for (let i = 0; i < numLeaves; i++) {
    currentLevelNodes[i] = nodeGen(
      { level: level, index: i, isLeaf: true, isRoot: false },
      []
    );
  }

  while (level > 0) {
    const nextLevelNodes: QWidget<any, any, any>[] = [];
    for (let i = 0; i < (currentLevelNodes.length>>1); i++) {
      const left = currentLevelNodes[i*2];
      const right = currentLevelNodes[i*2 + 1];
      nextLevelNodes.push(
        nodeGen({ level: level - 1, index: i, isLeaf: false, isRoot: false }, [
          left,
          right,
        ])
      );
    }
    currentLevelNodes = nextLevelNodes;
    level--;
  }
  const root = nodeGen(
    { level: 0, index: 0, isLeaf: false, isRoot: true },
    currentLevelNodes
  );
  return root;
}
function debugNodeGen(nid: ITreeNodeID) {
  return new QWRect({width: 100, height: 100, borderWidth: 2}, undefined, undefined, {label: `Node ${nid.level}-${nid.index}`, color: "#a55", textColor: "#fff", borderColor: "#000"});
}
const baseLayout = {
  direction: RectSide.Bottom,
  siblingSpacing: 30,
  levelSpacing: 20,
  parentAnchor: RectSide.Bottom,
  childAnchor: RectSide.Top,
};
function simpleNodeJunction(nid: ITreeNodeID, parent: QWidget<any, any, any>, children: QWidget<any, any, any>[]) {
  return QWTreeJunction.create(parent, children, {layout: baseLayout,});
}
function simpleTreeNodeGen(
  treeHeight: number,
  junctionGen: (
    nid: ITreeNodeID,
    parent: QWidget<any, any, any>,
    children: QWidget<any, any, any>[]
  ) => QWidget<any, any, any>,
  leafNodeGen: (nid: ITreeNodeID) => QWidget<any, any, any>,
  innerNodeGen: (nid: ITreeNodeID) => QWidget<any, any, any>,
  rootNodeGen: (nid: ITreeNodeID) => QWidget<any, any, any>
): QWidget<any, any, any> {
  return buildSimpleTree(treeHeight, (nid, children) => {
    console.log(nid, children);
    if (nid.isLeaf) {
      return leafNodeGen(nid);
    } else {
      const parent = nid.isRoot ? rootNodeGen(nid) : innerNodeGen(nid);
      if (!children.length) {
        return parent;
      } else {
        return junctionGen(nid, parent, children);
      }
    }
  });
}
function simpleDebugTree(treeHeight: number) {
  return simpleTreeNodeGen(treeHeight, simpleNodeJunction, debugNodeGen, debugNodeGen, debugNodeGen);
}

export {
  simpleTreeNodeGen,
  debugNodeGen,
  simpleDebugTree,
}