import { Node } from "@qstudio/flex-layout";

function getAllTabIdsForFilePath(root: Node, fileName: string) {
    const tabIds: string[] = [];
    const children = root.getChildren();
    for (const c of children) {
        if (c.getType() === "tab") {
            const cfg = (c as any).getConfig();
            if (cfg) {
                if (cfg.filePath === fileName) {
                    tabIds.push(c.getId());
                }
            }

        } else {
            tabIds.push(...getAllTabIdsForFilePath(c, fileName));
        }
    }
    return tabIds;

}
function getAllTabIdsForFilePaths(node: Node, filePaths: string[]): string[] {
    const tabIds: string[] = [];
    const children = node.getChildren();
    for (const c of children) {
        if (c.getType() === "tab") {
            const cfg = (c as any).getConfig();
            if (cfg && cfg.filePath) {
                if (filePaths.indexOf(cfg.filePath)) {
                    tabIds.push(c.getId());
                }
            }

        } else {
            tabIds.push(...getAllTabIdsForFilePaths(c, filePaths));
        }
    }
    return tabIds;

}

function getAllTabIdsForComponent(node: Node, component: string): string[] {
    const tabIds: string[] = [];
    const children = node.getChildren();
    for (const c of children) {
        if (c.getType() === "tab") {
            const cfg = (c as any).getConfig();
            if (cfg && cfg.getComponent) {
                if (cfg.getComponent() === component) {
                    tabIds.push(c.getId());
                }
            }
        } else {
            tabIds.push(...getAllTabIdsForComponent(c, component));
        }
    }
    return tabIds;

}

export {
    getAllTabIdsForFilePath,
    getAllTabIdsForFilePaths,
    getAllTabIdsForComponent,
}