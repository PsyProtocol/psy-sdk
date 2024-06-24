import { IQJobWithDependenciesSerialized } from "@qstudio/city-block";
import { ICSProofNode } from "./types";

function depSerializedToProofNodes(ser: IQJobWithDependenciesSerialized): ICSProofNode {
  return {
    id: ser.id,
    dependencies: ser.dependencies.map(depSerializedToProofNodes),
  };
}

export {
  depSerializedToProofNodes,
}