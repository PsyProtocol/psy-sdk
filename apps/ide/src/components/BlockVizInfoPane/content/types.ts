import { ICityJobWitness, IQProvingJobDataID, ProvingJobCircuitType } from "@qstudio/city-block";
import { TRichTextContent } from "../../RichTextRenderer/types";

interface IBlockVizJobInfoBase{
  jobIdHex: string;
  jobId: IQProvingJobDataID;
  dependencyJobs: string[];
  circuitType: ProvingJobCircuitType;


  title: string;
  description: string;
  summary: TRichTextContent;
  witness: ICityJobWitness;
  shortActions: TRichTextContent[];
  constraints: TRichTextContent[];


}

interface IBlockVizJobInfo<C extends keyof typeof ProvingJobCircuitType> extends IBlockVizJobInfoBase{
  circuitType: (typeof ProvingJobCircuitType)[C];
  witness: ICityJobWitness & {q_witness_type: C}
}
type IRealBlockVizJobInfo = IBlockVizJobInfo<keyof typeof ProvingJobCircuitType>;
export type {
  IBlockVizJobInfo,
  IBlockVizJobInfoBase,
  IRealBlockVizJobInfo,
};