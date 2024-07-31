import { ICRUserRegistrationCircuitInput, IQProvingJobDataID, IRegisterUserJobWitness, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo,  } from "../types";
import { CircuitDescriptions } from "../circuitInfo";
import { RichTextElemType } from "../../../RichTextRenderer/types";
import { generateAggStateBVJobInfo } from "./agg";

async function generateBVJobInfoRegisterUser(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"RegisterUser">> {
  const witness: IRegisterUserJobWitness = await ctx.rpc.getProofStoreJobWitness(jobIdHex);
  const userId = Math.floor(Number(witness.user_tree_delta_merkle_proof.index+"")/2);
  const publicKey = witness.user_tree_delta_merkle_proof.new_value;
  const jobInfo: IBlockVizJobInfo<"RegisterUser"> = {
    jobIdHex,
    jobId,
    circuitType: ProvingJobCircuitType.RegisterUser,
    witness,
    dependencyJobs: [],
    title: "Register User",
    description: CircuitDescriptions[ProvingJobCircuitType.RegisterUser],
    summary: [
      "Register ",
      {type: RichTextElemType.User, userId: userId+"", text: "User "+userId},
      " with public key ",
      {
        type: RichTextElemType.Hash,
        hash: publicKey,
        text: publicKey,
      }
    ],
    shortActions: [
      [
        "Register ",
        {type: RichTextElemType.User, userId: userId+"", text: "User "+userId},
      ],
    ],
    constraints: []
  };
  const w = jobInfo.witness;
  return jobInfo;
}



async function generateBVJobInfoRegisterUserAggregate(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"RegisterUserAggregate">> {
  const result = await generateAggStateBVJobInfo(ctx, jobIdHex, jobId, ["user"]);
  return result as IBlockVizJobInfo<"RegisterUserAggregate">;
}


export {
  generateBVJobInfoRegisterUser,
  generateBVJobInfoRegisterUserAggregate,
}

