import { ICRUserRegistrationCircuitInput, IQProvingJobDataID, IRegisterUserJobWitness, ITransferTokensL2JobWitness, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo,  } from "../types";
import { CircuitDescriptions } from "../circuitInfo";
import { RichTextElemType } from "../../../RichTextRenderer/types";
import { generateAggStateBVJobInfo } from "./agg";
import { hashOutToHex, hexToHashOut } from "packages/city-sdk/src/utils/data";
import { cityFeltSatsToDoge } from "@qstudio/city-sdk";
async function generateBVJobInfoTransferTokensL2(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"TransferTokensL2">> {
  const witness: ITransferTokensL2JobWitness = await ctx.rpc.getProofStoreJobWitness(jobIdHex);
  const senderUserId = Math.floor(Number(witness.sender_user_tree_delta_merkle_proof.index+"")/2);
  const receiverUserId = Math.floor(Number(witness.receiver_user_tree_delta_merkle_proof.index+"")/2);
  const senderOldBalance = hexToHashOut(witness.sender_user_tree_delta_merkle_proof.old_value)[0];
  const senderNewBalance = hexToHashOut(witness.sender_user_tree_delta_merkle_proof.new_value)[0];
  const transferAmount = cityFeltSatsToDoge((senderOldBalance-senderNewBalance))+" DOGE";

  const jobInfo: IBlockVizJobInfo<"TransferTokensL2"> = {
    jobIdHex,
    jobId,
    circuitType: ProvingJobCircuitType.TransferTokensL2,
    witness,
    dependencyJobs: [],
    title: "Transfer Tokens ",
    description: CircuitDescriptions[ProvingJobCircuitType.RegisterUser],
    summary: [
      "Transfer "+transferAmount+" from ",
      {type: RichTextElemType.User, userId: senderUserId+"", text: "User "+senderUserId},
      " to ",
      {type: RichTextElemType.User, userId: receiverUserId+"", text: "User "+receiverUserId}
    ],
    shortActions: [
      [
        "Transfer "+transferAmount+" from ",
        {type: RichTextElemType.User, userId: senderUserId+"", text: "User "+senderUserId},
        " to ",
        {type: RichTextElemType.User, userId: receiverUserId+"", text: "User "+receiverUserId}
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
  generateBVJobInfoTransferTokensL2,
  generateBVJobInfoRegisterUserAggregate,
}

