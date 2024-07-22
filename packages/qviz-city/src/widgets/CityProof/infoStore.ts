import { ProvingJobCircuitType, deserializeJobId } from "@qstudio/city-block";
import { CityProofStateType } from ".";
import { ICircuitIconDef, ICircuitIconStyle, ICityProofInfoStore, IProofIconHelper, IQVCityProofStyleDef, ProofWidgetStyleVariant } from "./types";
import { addDepositIcon, addWithdrawalIcon, aggregateAddDepositIcon, aggregateAddWithdrawalIcon, aggregateProcessDepositIcon, aggregateProcessWithdrawalIcon, aggregateRegisterUserIcon, aggregateStage1Icon, aggregateStage2Icon, aggregateTokenTransferIcon, finalSighashGroth16Icon, finalSighashIcon, genericAggregationIcon, processDepositIcon, processWithdrawalIcon, registerUserIcon, sighashIntrospectionIcon, stateTransitionIcon, tokenTransferIcon } from "./icons";

type TIconFactory = (styleConfig: ICircuitIconStyle) => ICircuitIconDef;
const DEFAULT_ICON: TIconFactory = genericAggregationIcon;


const IconDefs: Record<ProvingJobCircuitType, TIconFactory> = {
  [ProvingJobCircuitType.RegisterUser]: registerUserIcon,
  [ProvingJobCircuitType.RegisterUserAggregate]: aggregateRegisterUserIcon,
  [ProvingJobCircuitType.AddL1Deposit]: addDepositIcon,
  [ProvingJobCircuitType.AddL1DepositAggregate]: aggregateAddDepositIcon,
  [ProvingJobCircuitType.ClaimL1Deposit]: processDepositIcon,
  [ProvingJobCircuitType.ClaimL1DepositAggregate]: aggregateProcessDepositIcon,
  [ProvingJobCircuitType.TransferTokensL2]: tokenTransferIcon,
  [ProvingJobCircuitType.TransferTokensL2Aggregate]: aggregateTokenTransferIcon,
  [ProvingJobCircuitType.AddL1Withdrawal]: addWithdrawalIcon,
  [ProvingJobCircuitType.AddL1WithdrawalAggregate]: aggregateAddWithdrawalIcon,
  [ProvingJobCircuitType.ProcessL1Withdrawal]: processWithdrawalIcon,
  [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: aggregateProcessWithdrawalIcon,
  [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: stateTransitionIcon,
  [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: sighashIntrospectionIcon,
  [ProvingJobCircuitType.GenerateFinalSigHashProof]: finalSighashIcon,
  [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: finalSighashGroth16Icon,
  [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: finalSighashGroth16Icon,
  [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: aggregateStage1Icon,
  [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]: aggregateStage2Icon,
  [ProvingJobCircuitType.DummyRegisterUserAggregate]: registerUserIcon,
  [ProvingJobCircuitType.DummyAddL1DepositAggregate]: addDepositIcon,
  [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: processDepositIcon,
  [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: tokenTransferIcon,
  [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: addWithdrawalIcon,
  [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]: processWithdrawalIcon,
  [ProvingJobCircuitType.WrappedSignatureProof]: DEFAULT_ICON,
  [ProvingJobCircuitType.Secp256K1SignatureProof]: DEFAULT_ICON,
  [ProvingJobCircuitType.Unknown]: DEFAULT_ICON
}

const VariantDefs: Record<ProvingJobCircuitType, ProofWidgetStyleVariant> = {
  [ProvingJobCircuitType.RegisterUser]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.RegisterUserAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.AddL1Deposit]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.AddL1DepositAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.ClaimL1Deposit]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.ClaimL1DepositAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.TransferTokensL2]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.TransferTokensL2Aggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.AddL1Withdrawal]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.AddL1WithdrawalAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.ProcessL1Withdrawal]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.GenerateFinalSigHashProof]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: ProofWidgetStyleVariant.BigAggregate,
  [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]: ProofWidgetStyleVariant.BigAggregate,
  [ProvingJobCircuitType.DummyRegisterUserAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.DummyAddL1DepositAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]: ProofWidgetStyleVariant.Aggregate,
  [ProvingJobCircuitType.WrappedSignatureProof]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.Secp256K1SignatureProof]: ProofWidgetStyleVariant.Standard,
  [ProvingJobCircuitType.Unknown]: ProofWidgetStyleVariant.Standard
}

function getCircuitIcon(circuitType: ProvingJobCircuitType, iconRootClass: string, styleConfig: ICircuitIconStyle): ICircuitIconDef {
  const inner = IconDefs[circuitType](styleConfig);
  const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
  g.setAttribute("class", iconRootClass + " circuitIcon-" + ProvingJobCircuitType[circuitType]);
  g.appendChild(inner.g);
  return {
    width: inner.width,
    height: inner.height,
    g,
  };
}

class SimpleCityProofInfoStore implements ICityProofInfoStore {
  getProofWidgetVariantForJob(jobId: string): ProofWidgetStyleVariant {
    return VariantDefs[deserializeJobId(jobId).circuit_type];
  }
  getProofIconForJob(jobId: string, styleDef: IQVCityProofStyleDef): IProofIconHelper {
    const g = getCircuitIcon(deserializeJobId(jobId).circuit_type, styleDef.iconRoot, styleDef.styleConfig);

    const helper: IProofIconHelper = {
      setState: (_: CityProofStateType) => {
      },
      getSize: () => {
        return {
          width: g.width,
          height: g.height,
        };
      },
      getGroup: () => {
        return g.g;
      }
    };
    return helper;

  }
  circuitTypeProvingTimes: Record<ProvingJobCircuitType, number> = {
    [ProvingJobCircuitType.RegisterUser]: 501,
    [ProvingJobCircuitType.RegisterUserAggregate]: 501,
    [ProvingJobCircuitType.AddL1Deposit]: 501,
    [ProvingJobCircuitType.AddL1DepositAggregate]: 501,
    [ProvingJobCircuitType.ClaimL1Deposit]: 501,
    [ProvingJobCircuitType.ClaimL1DepositAggregate]: 501,
    [ProvingJobCircuitType.TransferTokensL2]: 501,
    [ProvingJobCircuitType.TransferTokensL2Aggregate]: 501,
    [ProvingJobCircuitType.AddL1Withdrawal]: 501,
    [ProvingJobCircuitType.AddL1WithdrawalAggregate]: 501,
    [ProvingJobCircuitType.ProcessL1Withdrawal]: 501,
    [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: 501,
    [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: 501,
    [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: 501,
    [ProvingJobCircuitType.GenerateFinalSigHashProof]: 501,
    [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: 501,
    [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: 501,
    [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: 501,
    [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]: 501,
    [ProvingJobCircuitType.DummyRegisterUserAggregate]: 501,
    [ProvingJobCircuitType.DummyAddL1DepositAggregate]: 501,
    [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: 501,
    [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: 501,
    [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: 501,
    [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]: 501,
    [ProvingJobCircuitType.WrappedSignatureProof]: 501,
    [ProvingJobCircuitType.Secp256K1SignatureProof]: 501,
    [ProvingJobCircuitType.Unknown]: 0
  };
  getProvingTime(jobId: string): number {
    return this.circuitTypeProvingTimes[deserializeJobId(jobId).circuit_type];
  }
}
const globalProofInfoStore = new SimpleCityProofInfoStore();

export {
  SimpleCityProofInfoStore,
  globalProofInfoStore,
}
export type {
  IProofIconHelper,
  ICityProofInfoStore,
}