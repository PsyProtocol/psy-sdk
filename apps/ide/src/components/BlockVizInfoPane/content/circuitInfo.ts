import { ICRAddL1DepositCircuitInput, ICRUserRegistrationCircuitInput, IQProvingJobDataID, ProvingJobCircuitType } from "@qstudio/city-block";
interface ICircuitDisplayInfo {
  description: string;
} 
const CircuitDescriptions: Record<ProvingJobCircuitType, string> = {
  [ProvingJobCircuitType.RegisterUser]: "Proves that the user tree legally transitions from its current root to a new valid root after adding a user to the user's tree with a given public key and with balance/nonce initialized to 0.",
  [ProvingJobCircuitType.RegisterUserAggregate]: "Aggregates two RegisterUser/RegisterUserAggregate proofs into a single proof that proves their combined state trasition within the user tree.",
  [ProvingJobCircuitType.AddL1Deposit]: "Proves that a new deposit is legally added to the deposit tree in the next empty slot and that this action results in a valid deposit event hash in the public inputs of the proof",
  [ProvingJobCircuitType.AddL1DepositAggregate]: "Aggregates two AddL1Deposit/AddL1DepositAggregate proofs into a single proof that proves their combined state trasition within the deposit tree, as well as, generating a new valid deposit event hash which reflects the combined newly added deposits.",
  [ProvingJobCircuitType.ClaimL1Deposit]: "Proves that a user claims a valid deposit in the deposits tree (henceforth setting the deposit leaf to 0), increments the user's balance by the deposit amount, and checks that the user provided a valid signature for the deposit claim signed with the same secp256k1 public key used to make the deposit on L1.",
  [ProvingJobCircuitType.ClaimL1DepositAggregate]: "Aggregates two ClaimL1Deposit/ClaimL1DepositAggregate proofs into a single proof that proves their combined state trasition within the deposit/user tree.",
  [ProvingJobCircuitType.TransferTokensL2]: "Proves that a user transfers tokens from their balance to another user's balance, updating the user's balance and nonce in the user tree, as well as proving that the sender has signed a message with the correct key to authorize the transfer.",
  [ProvingJobCircuitType.TransferTokensL2Aggregate]: "Aggregates two TransferTokensL2/TransferTokensL2Aggregate proofs into a single proof that proves their combined state trasition within the user tree.",
  [ProvingJobCircuitType.AddL1Withdrawal]: "Proves that a new withdrawal is legally added to the withdrawal tree in the next empty slot, that the user's balance is correctly updated and that the user has signed a valid request with the zk private key.",
  [ProvingJobCircuitType.AddL1WithdrawalAggregate]: "Aggregates two AddL1Withdrawal/AddL1WithdrawalAggregate proofs into a single proof that proves their combined state transitions within the user and withdrawal tree",
  [ProvingJobCircuitType.ProcessL1Withdrawal]: "Proves that the first non-empty withdrawal leaf in the withdrawals tree is cleared and that the resulting withdrawal events hash is updating accordingly.",
  [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: "Aggregates two ProcessL1Withdrawal/ProcessL1WithdrawalAggregate proofs into a single proof that proves their combined state trasition within the withdrawals tree, as well as, generating a new valid withdrawal event hash which reflects their combined processed withdrawals.",
  [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: "Proves that the state root of the layer 2 legally transitions from a start state hash to a new state hash, as well as that the transition occured with deposits and withdrawals relfected in their corresponding events hash.",
  [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: "Proves that a block spend transaction with a given sighash is the result of a state transition from state root R1 to R2, that this transition is a result of deposits reflected in a deposits event hash, withdrawals reflected in a withdrawals event hash, and ensures that the verifier data for the proof in the block spend output of the UTXO has the same verifier data + the new state root hard coded in its P2SH script.",
  [ProvingJobCircuitType.GenerateFinalSigHashProof]: "Recursively verifies a block state transition proof and a sighash introspection proof, ensuring that both agree on the state transition, added deposits and processed withdrawals for the block (proves that a block script spend with a given sighash is valid)",
  [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: "Generates a Groth16 proof which recursively verifies a GenerateFinalSigHashProof, and has 2 public inputs: the state state root from the sighash proof and the expected sighash.",
  [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: "Generates a Groth16 proof which recursively verifies a GenerateFinalSigHashProof, and has 2 public inputs: the state state root from the sighash proof and the expected sighash.",
  [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: "Aggregates proofs from all user registrations, deposit claims, and token transfers in the block (proves that these actions result in valid state transitions for the user and deposit tree)",
  [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]: "Aggregates proofs from all added/processed withdrawals and added withdrawals in the block, and proves that these events result in a given deposit event hash + withdrawals event hash.",
  [ProvingJobCircuitType.DummyRegisterUserAggregate]: "A \"dummy\" proof which proves that the user tree can legally transition from its current root to its current root (proves that its ok not to update the user tree).",
  [ProvingJobCircuitType.DummyAddL1DepositAggregate]: "A \"dummy\" proof which proves that the deposit tree can legally transition from its current root to its current root (proves that its ok not to update the deposit tree).",
  [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: "A \"dummy\" proof which proves that the deposit+user tree can legally transition from their current roots to their current roots (proves that its ok not to update the deposit/user tree).",
  [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: "A \"dummy\" proof which proves that the user tree can legally transition from its current root to its current root (proves that its ok not to update the user tree).",
  [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: "A \"dummy\" proof which proves that the withdrawals/user tree can legally transition from their current roots to its current roots (i.e neither tree is updated, and thats ok to have in valid block).",
  [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]:"A \"dummy\" proof which proves that the withdrawals/user tree can legally transition from their current roots to its current roots (i.e neither tree is updated, and thats ok to have in valid block).",
  [ProvingJobCircuitType.WrappedSignatureProof]: "",
  [ProvingJobCircuitType.Secp256K1SignatureProof]: "",
  [ProvingJobCircuitType.Unknown]: ""
}

/*
const CircuitActionSummaries: Record<ProvingJobCircuitType, (witness: any, jobId: IQProvingJobDataID,)=>string> = {
  [ProvingJobCircuitType.RegisterUser]:(witness: ICRUserRegistrationCircuitInput)=>{
    const userId = Math.floor(Number(witness.user_tree_delta_merkle_proof.index)/2);
    return `Register User ${userId} with a public key of ${witness.user_tree_delta_merkle_proof.new_value}`;
  },
  [ProvingJobCircuitType.RegisterUserAggregate]: ()=> "",
  [ProvingJobCircuitType.AddL1Deposit]: (witness: ICRAddL1DepositCircuitInput)=>{
    return `Add a deposit of ${witness.deposit_tree_delta_merkle_proof.new_value} to the deposit tree`;
  },
  [ProvingJobCircuitType.AddL1DepositAggregate]: ()=> "",
  [ProvingJobCircuitType.ClaimL1Deposit]: "Proves that a user claims a valid deposit in the deposits tree (henceforth setting the deposit leaf to 0), increments the user's balance by the deposit amount, and checks that the user provided a valid signature for the deposit claim signed with the same secp256k1 public key used to make the deposit on L1.",
  [ProvingJobCircuitType.ClaimL1DepositAggregate]:  ()=> "",
  [ProvingJobCircuitType.TransferTokensL2]: "Proves that a user transfers tokens from their balance to another user's balance, updating the user's balance and nonce in the user tree, as well as proving that the sender has signed a message with the correct key to authorize the transfer.",
  [ProvingJobCircuitType.TransferTokensL2Aggregate]:  ()=> "",
  [ProvingJobCircuitType.AddL1Withdrawal]: "Proves that a new withdrawal is legally added to the withdrawal tree in the next empty slot, that the user's balance is correctly updated and that the user has signed a valid request with the zk private key.",
  [ProvingJobCircuitType.AddL1WithdrawalAggregate]: ()=> "",
  [ProvingJobCircuitType.ProcessL1Withdrawal]: "Proves that the first non-empty withdrawal leaf in the withdrawals tree is cleared and that the resulting withdrawal events hash is updating accordingly.",
  [ProvingJobCircuitType.ProcessL1WithdrawalAggregate]: ()=> "",
  [ProvingJobCircuitType.GenerateRollupStateTransitionProof]: ()=>"",
  [ProvingJobCircuitType.GenerateSigHashIntrospectionProof]: ()=>"",
  [ProvingJobCircuitType.GenerateFinalSigHashProof]: ()=>"",
  [ProvingJobCircuitType.GenerateFinalSigHashProofGroth16]: ()=>"",
  [ProvingJobCircuitType.WrapFinalSigHashProofBLS12381]: ()=>"",
  [ProvingJobCircuitType.AggUserRegisterClaimDepositL2Transfer]: ()=>"",
  [ProvingJobCircuitType.AggAddProcessL1WithdrawalAddL1Deposit]:()=>"",
  [ProvingJobCircuitType.DummyRegisterUserAggregate]: ()=> "",
  [ProvingJobCircuitType.DummyAddL1DepositAggregate]: ()=> "",
  [ProvingJobCircuitType.DummyClaimL1DepositAggregate]: ()=> "",
  [ProvingJobCircuitType.DummyTransferTokensL2Aggregate]: ()=> "",
  [ProvingJobCircuitType.DummyAddL1WithdrawalAggregate]: ()=> "",
  [ProvingJobCircuitType.DummyProcessL1WithdrawalAggregate]: ()=>"",
  [ProvingJobCircuitType.WrappedSignatureProof]: ()=>"",
  [ProvingJobCircuitType.Secp256K1SignatureProof]: ()=>"",
  [ProvingJobCircuitType.Unknown]: ()=>"",
}
*/
export {
  CircuitDescriptions,
}