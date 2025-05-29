import { ICityAggregatedOpJobCircuitType } from "./types";
import { IQProvingJobDataID } from "../job/id";

interface IOpTreeAggProofCategory {
    triplet: ICityAggregatedOpJobCircuitType;
    levels: IQProvingJobDataID[][];
    root: IQProvingJobDataID;
}
interface IProofCategories {
    treeAgg: IOpTreeAggProofCategory[];
}

export type { IOpTreeAggProofCategory };
