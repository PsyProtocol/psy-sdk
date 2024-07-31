import { IClaimL1DepositJobWitness, ICRUserRegistrationCircuitInput, IQProvingJobDataID, IRegisterUserJobWitness, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo,  } from "../types";
import { CircuitDescriptions } from "../circuitInfo";
import { RichTextElemType } from "../../../RichTextRenderer/types";
import { generateAggStateBVJobInfo } from "./agg";
import { cityFeltSatsToDoge } from "@qstudio/city-sdk";

async function generateBVJobInfoClaimL1Deposit(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"ClaimL1Deposit">> {
  const witness: IClaimL1DepositJobWitness = await ctx.rpc.getProofStoreJobWitness(jobIdHex);
  const depositId = Number(witness.deposit_tree_delta_merkle_proof.index+"");
  const userId = Math.floor(Number(witness.user_tree_delta_merkle_proof.index+"")/2);
  const publicKey = witness.user_tree_delta_merkle_proof.new_value;
  const deposit = await ctx.rpc.getDepositById(ctx.checkpoint_id, depositId);
  const jobInfo: IBlockVizJobInfo<"ClaimL1Deposit"> = {
    jobIdHex,
    jobId,
    circuitType: ProvingJobCircuitType.ClaimL1Deposit,
    witness,
    dependencyJobs: [],
    title: "Claim Deposit",
    description: CircuitDescriptions[ProvingJobCircuitType.ClaimL1Deposit],
    summary: [
      "Claim ",
      {type: RichTextElemType.Deposit, depositId: depositId+"", text: "Deposit "+depositId},
      " with transaction id ",
      {type: RichTextElemType.TransactionId, txid: deposit.txid, text: deposit.txid},
      " and amount "+cityFeltSatsToDoge(deposit.value)+" DOGE for ",
      {type: RichTextElemType.User, userId: userId+"", text: "User "+userId},
    ],
    shortActions: [
      [
        "Claim ",
        {type: RichTextElemType.Deposit, depositId: depositId+"", text: "Deposit "+depositId},
        " ("+cityFeltSatsToDoge(deposit.value)+" DOGE)"
      ],
    ],
    constraints: []
  };
  const w = jobInfo.witness;
  return jobInfo;
}



async function generateBVJobInfoClaimL1DepositAggregate(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID): Promise<IBlockVizJobInfo<"ClaimL1DepositAggregate">> {
  const result = await generateAggStateBVJobInfo(ctx, jobIdHex, jobId, ["deposit","user"]);
  return result as IBlockVizJobInfo<"ClaimL1DepositAggregate">;
}


export {
  generateBVJobInfoClaimL1Deposit,
  generateBVJobInfoClaimL1DepositAggregate,
}

