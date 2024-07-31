import { deserializeJobId, IQProvingJobDataID, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { TBVJobInfoGenerator } from "./types";
import { IRealBlockVizJobInfo, IBlockVizJobInfo } from "../types";
import { generatePlaceHolderBVJobInfo } from "./placeholder";
import { generateBVJobInfoRegisterUser } from "./UserRegistration";
import { wrapGenerateAggStateBVJobInfo, wrapGenerateAggWithEventsStateBVJobInfo } from "./agg";
import { generateBVJobInfoClaimL1Deposit } from "./ClaimDeposit";
import { generateBVJobInfoTransferTokensL2 } from "./TokenTransfer";
import { generateBVJobInfoAddL1Deposit } from "./AddDeposit";

const BlockVizCircuitContentGenerators: Record<ProvingJobCircuitType, TBVJobInfoGenerator> = {
  [ProvingJobCircuitType.RegisterUser]: generateBVJobInfoRegisterUser,
  [ProvingJobCircuitType.RegisterUserAggregate]: wrapGenerateAggStateBVJobInfo(["user"]),
  [ProvingJobCircuitType.AddL1Deposit]: generateBVJobInfoAddL1Deposit,
  [ProvingJobCircuitType.AddL1DepositAggregate]: wrapGenerateAggWithEventsStateBVJobInfo(["deposit"], ["deposit"]),
  [ProvingJobCircuitType.ClaimL1Deposit]: generateBVJobInfoClaimL1Deposit,
  [ProvingJobCircuitType.ClaimL1DepositAggregate]: wrapGenerateAggStateBVJobInfo(["deposit","user"]),
  [ProvingJobCircuitType.TransferTokensL2]: generateBVJobInfoTransferTokensL2,
  [ProvingJobCircuitType.TransferTokensL2Aggregate]: wrapGenerateAggStateBVJobInfo(["user"]),
  [ProvingJobCircuitType.AddL1Withdrawal]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.AddL1WithdrawalAggregate]: wrapGenerateAggStateBVJobInfo(["user", "withdrawal"]),
  [ProvingJobCircuitType.ProcessL1Withdrawal]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: wrapGenerateAggWithEventsStateBVJobInfo(["withdrawal"],["withdrawal"]),
  [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.GenerateFinalSigHashProof]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyRegisterUserAggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyAddL1DepositAggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.WrappedSignatureProof]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.Secp256K1SignatureProof]: generatePlaceHolderBVJobInfo,
  [ProvingJobCircuitType.Unknown]: generatePlaceHolderBVJobInfo
}


async function getCircuitContentInfo(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string){
  const decoded = deserializeJobId(jobIdHex);
  const generator = BlockVizCircuitContentGenerators[decoded.circuit_type];
  return generator(ctx, jobIdHex, decoded);
}

export {
  BlockVizCircuitContentGenerators,
  getCircuitContentInfo,
}