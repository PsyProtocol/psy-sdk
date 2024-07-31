import { IQProvingJobDataID, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo } from "../types";

type TBVJobInfoGenerator = (ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID) => Promise<IBlockVizJobInfo<keyof typeof ProvingJobCircuitType>>;

export type {TBVJobInfoGenerator};