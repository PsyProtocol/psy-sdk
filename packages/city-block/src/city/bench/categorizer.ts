import { IQProvingJobDataID } from "../job/id";
import { ICityAggregatedOpJobCircuitType } from "./types";

interface IOpTreeAggProofCategory {
  triplet: ICityAggregatedOpJobCircuitType;
  levels: IQProvingJobDataID[][];
  root: IQProvingJobDataID;
}
interface IProofCategories {

  treeAgg: IOpTreeAggProofCategory[];
  

}

export type {
  IOpTreeAggProofCategory,
}