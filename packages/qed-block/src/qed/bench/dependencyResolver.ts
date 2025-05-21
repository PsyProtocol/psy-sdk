import { deserializeJobId, getJobWitnessIdHex } from "../job";
import { ICSProofNode, IQJobWithDependencies, IQJobWithDependenciesSerialized } from "./types";
function walkIQJobWithDependenciesSerialized(
    root: IQJobWithDependenciesSerialized,
    visitor: (node: IQJobWithDependenciesSerialized) => void
) {
    visitor(root);
    root.dependencies.forEach((x) => walkIQJobWithDependenciesSerialized(x, visitor));
}
function walkIQJobWithDependencies(root: IQJobWithDependencies, visitor: (node: IQJobWithDependencies) => void) {
    visitor(root);
    root.dependencies.forEach((x) => walkIQJobWithDependencies(x, visitor));
}
function generateDependencyMappingResultSerialized(root: IQJobWithDependenciesSerialized) {
    const dMap: Record<string, string[]> = {};

    walkIQJobWithDependenciesSerialized(root, (n) => {
        const deps = n.dependencies.map((x) => x.id);
        if (Array.isArray(dMap[n.id])) {
            dMap[n.id] = Array.from(new Set([...dMap[n.id], ...deps]));
        } else {
            dMap[n.id] = deps;
        }
    });
    return dMap;
}

function depSerializedToProofNodes(ser: IQJobWithDependenciesSerialized): ICSProofNode {
    return {
        id: getJobWitnessIdHex(ser.id),
        dependencies: ser.dependencies.map(depSerializedToProofNodes),
    };
}

export {
    generateDependencyMappingResultSerialized,
    walkIQJobWithDependencies,
    walkIQJobWithDependenciesSerialized,
    depSerializedToProofNodes,
};
