import { IAddL1DepositJobWitness, ICRUserRegistrationCircuitInput, IQProvingJobDataID, IRegisterUserJobWitness, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo,  } from "../types";
import { CircuitDescriptions } from "../circuitInfo";
import { RichTextElemType } from "../../../RichTextRenderer/types";
import { generateAggStateBVJobInfo, generateAggWithEventsStateBVJobInfo } from "./agg";
import { cityFeltSatsToDoge } from "@qstudio/city-sdk";

async function generateBVJobInfoAddL1Deposit(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"AddL1Deposit">> {
  const witness: IAddL1DepositJobWitness = await ctx.rpc.getProofStoreJobWitness(jobIdHex);
  const depositId = Number(witness.deposit_tree_delta_merkle_proof.index+"");
  const deposit = await ctx.rpc.getDepositById(ctx.checkpoint_id, depositId);
  const jobInfo: IBlockVizJobInfo<"AddL1Deposit"> = {
    jobIdHex,
    jobId,
    circuitType: ProvingJobCircuitType.AddL1Deposit,
    witness,
    dependencyJobs: [],
    title: "Add Deposit",
    description: CircuitDescriptions[ProvingJobCircuitType.AddL1Deposit],
    summary: [
      "Add ",
      {type: RichTextElemType.Deposit, depositId: depositId+"", text: "Deposit  "+depositId},
      " with transaction id ",
      {type: RichTextElemType.TransactionId, txid: deposit.txid, text: deposit.txid},
      " and amount "+cityFeltSatsToDoge(deposit.value)+" DOGE to the deposits tree.",
      {type: RichTextElemType.LineBreak},
      {type: RichTextElemType.LineBreak},
      "Register deposit event with event hash ",
      {type: RichTextElemType.Hash, text: witness.deposit_tree_delta_merkle_proof.new_value, hash: witness.deposit_tree_delta_merkle_proof.new_value},
    ],
    shortActions: [
      [
        "Add ",
        {type: RichTextElemType.Deposit, depositId: depositId+"", text: "Deposit "+depositId},
        " ("+cityFeltSatsToDoge(deposit.value)+" DOGE)"
      ],
    ],
    constraints: []
  };
  return jobInfo;
}



async function generateBVJobInfoAddL1DepositAggregate(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"AddL1DepositAggregate">> {
  const result = await generateAggWithEventsStateBVJobInfo(ctx, jobIdHex, jobId, ["deposit"], ["deposit"]);
  return result as IBlockVizJobInfo<"AddL1DepositAggregate">;
}


export {
  generateBVJobInfoAddL1Deposit,
  generateBVJobInfoAddL1DepositAggregate,
}

