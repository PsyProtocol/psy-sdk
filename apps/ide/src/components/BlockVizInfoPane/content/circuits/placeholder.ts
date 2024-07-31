import { getCircuitNameForJobId, IAggStateTransitionInput, IAggStateTransitionWithEventsInput, IQProvingJobDataID, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo } from "../types";
import { RichTextElemType, TRichTextContent } from "../../../RichTextRenderer/types";
import { CircuitDescriptions } from "../circuitInfo";
import {twoToOneHex} from "poseidon-goldilocks-lite";


async function generatePlaceHolderBVJobInfo(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<keyof typeof ProvingJobCircuitType>> {
  const witness = await ctx.loadJobWitness(jobIdHex);


  const deps = ctx.getDirectDependencyJobs(jobIdHex);
  


  const summary: TRichTextContent = [];

  const depsShortActions = (await Promise.all(deps.map(async dep => {
    const ji = await ctx.loadJobInfo(dep);
    return ji.shortActions;
  }))).flatMap(x=>x);
  console.log("d,ds",deps,depsShortActions);


  const jobInfo: IBlockVizJobInfo<keyof typeof ProvingJobCircuitType> = {
    jobIdHex,
    jobId,
    circuitType: jobId.circuit_type,
    witness,
    dependencyJobs: deps,
    title: getCircuitNameForJobId(jobId),
    description: CircuitDescriptions[jobId.circuit_type],
    summary: summary,
    shortActions: depsShortActions,
    constraints: []
  };
  return jobInfo;

}

export {
  generatePlaceHolderBVJobInfo,
}