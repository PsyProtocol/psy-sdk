import { ICityAggregatedOpJobCircuitType } from "../bench/types";
import { IQProvingJobDataID, ProvingJobCircuitType } from "./id";

const LeafOpCircuits: ProvingJobCircuitType[] = [
  ProvingJobCircuitType.RegisterUser,
  ProvingJobCircuitType.AddL1Deposit,
  ProvingJobCircuitType.ClaimL1Deposit,
  ProvingJobCircuitType.TransferTokensL2,
  ProvingJobCircuitType.AddL1Withdrawal,
  ProvingJobCircuitType.ProcessL1Withdrawal,
];
const AggOpCircuits: ProvingJobCircuitType[] = [
  ProvingJobCircuitType.RegisterUserAggregate,
  ProvingJobCircuitType.AddL1DepositAggregate,
  ProvingJobCircuitType.ClaimL1DepositAggregate,
  ProvingJobCircuitType.TransferTokensL2Aggregate,
  ProvingJobCircuitType.AddL1WithdrawalAggregate,
  ProvingJobCircuitType.ProcessL1WithdrawalAggregate,
];

const DummyOpCircuits: ProvingJobCircuitType[] = [
  ProvingJobCircuitType.DummyRegisterUserAggregate,
  ProvingJobCircuitType.DummyAddL1DepositAggregate,
  ProvingJobCircuitType.DummyClaimL1DepositAggregate,
  ProvingJobCircuitType.DummyTransferTokensL2Aggregate,
  ProvingJobCircuitType.DummyAddL1WithdrawalAggregate,
  ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate,
];

const AggOpTriplets: ICityAggregatedOpJobCircuitType[] = LeafOpCircuits.map((l, i)=>{
  return {
    leaf: l,
    agg: AggOpCircuits[i],
    dummy: DummyOpCircuits[i],
    has_events: l === ProvingJobCircuitType.AddL1Deposit || l == ProvingJobCircuitType.ProcessL1Withdrawal,
  }
});

function getAggOpIndexForCircuitType(circuitType: ProvingJobCircuitType): number {
  const leafIndex = LeafOpCircuits.indexOf(circuitType);
  if(leafIndex !== -1){
    return leafIndex;
  }
  const aggIndex = AggOpCircuits.indexOf(circuitType);
  if(aggIndex !== -1){
    return aggIndex;
  }
  const dummyIndex = DummyOpCircuits.indexOf(circuitType);
  if(dummyIndex !== -1){
    return dummyIndex;
  }
  return -1;
}

function getAggOpTripletForCircuitType(circuitType: ProvingJobCircuitType): ICityAggregatedOpJobCircuitType | null {
  const index = getAggOpIndexForCircuitType(circuitType);
  if(index === -1){
    return null;
  }
  return AggOpTriplets[index];
}


function isDummyOpJob(id: IQProvingJobDataID): boolean {
  return DummyOpCircuits.indexOf(id.circuit_type) !== -1
}
function isLeafOpJob(id: IQProvingJobDataID): boolean {
  return LeafOpCircuits.indexOf(id.circuit_type) !== -1
}
function isAggOpJob(id: IQProvingJobDataID): boolean {
  return AggOpCircuits.indexOf(id.circuit_type) !== -1;
}
function isAggregatorForChild(parent: IQProvingJobDataID, child: IQProvingJobDataID): boolean {
  if(isAggOpJob(parent) && isLeafOpJob(child) && parent.circuit_type + 1 === child.circuit_type){
    return true;
  }else if(isAggOpJob(parent) && isAggOpJob(child) && parent.circuit_type === child.circuit_type){
    return true;
  }else{
    return false;
  }
}



export {
  isLeafOpJob,
  isAggOpJob,
  isAggregatorForChild,
  isDummyOpJob,
  getAggOpIndexForCircuitType,
  getAggOpTripletForCircuitType,
  AggOpCircuits,
  LeafOpCircuits,
  DummyOpCircuits,
}
