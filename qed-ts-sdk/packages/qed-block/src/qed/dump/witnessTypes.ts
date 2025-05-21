import { CityDeltaMerkleProof, CityMerkleProof } from "@qed/qed-ts-sdk";
import { IQProvingJobDataID, ProvingJobCircuitType } from "../job/id";
import { IBlockSpendIntrospectionHint } from "./serializedTx";
type SCFelt = bigint | number;

type CityHash = string;
interface ICRSigHashWrapperCircuitInput {
    introspection_hint: IBlockSpendIntrospectionHint;
    whitelist_inclusion_proof: CityMerkleProof;
}
interface IBTCRollupIntrospectionFinalizedResult {
    deposits_hash: CityHash;
    withdrawals_hash: CityHash;

    current_block_state_hash: CityHash;
    next_block_state_hash: CityHash;

    total_deposits_count: SCFelt;
    total_withdrawals_count: SCFelt;

    total_deposits_value: SCFelt;
    total_withdrawals_value: SCFelt;
    current_block_rollup_balance: SCFelt;
    next_block_rollup_balance: SCFelt;
}

interface ICRSigHashFinalGLCircuitInput {
    result: IBTCRollupIntrospectionFinalizedResult;
    state_transition_proof_id: IQProvingJobDataID;
    sighash_introspection_proof_id: IQProvingJobDataID;
}
interface IBTCRollupIntrospectionResultDeposit {
    public_key: SCFelt[];
    txid_224: CityHash;
    value: number;
}

interface ICRAddL1DepositCircuitInput {
    deposit_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
}

interface ICRAddL1WithdrawalCircuitInput {
    user_tree_delta_merkle_proof: CityDeltaMerkleProof;
    withdrawal_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
    signature_proof_id: IQProvingJobDataID;
}

interface ICRClaimL1DepositCircuitInput {
    deposit: IBTCRollupIntrospectionResultDeposit;
    user_tree_delta_merkle_proof: CityDeltaMerkleProof;
    deposit_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
    signature_proof_id: IQProvingJobDataID;
}

interface ICRL2TransferCircuitInput {
    sender_user_tree_delta_merkle_proof: CityDeltaMerkleProof;
    receiver_user_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
    signature_proof_id: IQProvingJobDataID;
}

interface ICRProcessL1WithdrawalCircuitInput {
    withdrawal_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
}

interface ICRUserRegistrationCircuitInput {
    user_tree_delta_merkle_proof: CityDeltaMerkleProof;
    allowed_circuit_hashes_root: CityHash;
}

interface IDummyAggStateTransition {
    state_transition_hash: CityHash;
    allowed_circuit_hashes_root: CityHash;
}

interface IDummyAggStateTransitionWithEvents {
    state_transition_hash: CityHash;
    event_transition_hash: CityHash;
    allowed_circuit_hashes_root: CityHash;
}

interface IAggStateTransition {
    state_transition_start: CityHash;
    state_transition_end: CityHash;
}

interface IAggStateTransitionInput {
    left_input: IAggStateTransition;
    right_input: IAggStateTransition;
    left_proof_is_leaf: boolean;
    right_proof_is_leaf: boolean;
}

interface IAggStateTransitionWithEvents {
    state_transition_start: CityHash;
    state_transition_end: CityHash;
    event_hash: CityHash;
}

interface IAggStateTransitionWithEventsInput {
    left_input: IAggStateTransitionWithEvents;
    right_input: IAggStateTransitionWithEvents;
    left_proof_is_leaf: boolean;
    right_proof_is_leaf: boolean;
}

interface ICRAggUserRegisterClaimDepositL2TransferCircuitInput {
    op_register_user_transition_user_state_tree: IAggStateTransition;
    op_register_user_proof_id: IQProvingJobDataID;
    op_claim_l1_deposit_transition_deposit_tree: IAggStateTransition;
    op_claim_l1_deposit_transition_user_state_tree: IAggStateTransition;
    op_claim_l1_deposit_proof_id: IQProvingJobDataID;
    op_l2_transfer_transition_user_state_tree: IAggStateTransition;
    op_l2_transfer_proof_id: IQProvingJobDataID;
}

interface ICRAggUserRegisterClaimDepositL2TransferStateTransition {
    user_state_tree_transition: IAggStateTransition;
    deposit_tree_transition: IAggStateTransition;
    proof_id: IQProvingJobDataID;
}

interface ICRAggAddProcessL1WithdrawalAddL1DepositCircuitInput {
    op_add_l1_withdrawal_transition_user_state_tree: IAggStateTransition;
    op_add_l1_withdrawal_transition_withdrawal_tree: IAggStateTransition;
    op_add_l1_withdrawal_proof_id: IQProvingJobDataID;
    op_process_l1_withdrawal_transition_withdrawal_tree: IAggStateTransition;
    op_process_l1_withdrawal_proof_id: IQProvingJobDataID;
    op_add_l1_deposit_transition_deposit_tree: IAggStateTransition;
    op_add_l1_deposit_proof_id: IQProvingJobDataID;
}

interface ICRAggAddProcessL1WithdrawalAddL1DepositStateTransition {
    user_state_tree_transition: IAggStateTransition;
    withdrawal_tree_transition: IAggStateTransition;
    deposit_tree_transition: IAggStateTransition;
    proof_id: IQProvingJobDataID;
}

interface ICRBlockStateTransitionCircuitInput {
    agg_user_register_claim_deposits_l2_transfer: ICRAggUserRegisterClaimDepositL2TransferStateTransition;
    agg_add_process_withdrawals_add_l1_deposit: ICRAggAddProcessL1WithdrawalAddL1DepositStateTransition;
}

interface ICRAggL2TransferAddL1WithdrawalCircuitInput {
    op_l2_transfer_transition_user_state_tree: IAggStateTransition;
    op_l2_transfer_proof_id: IQProvingJobDataID;
    op_add_l1_withdrawal_transition_withdrawal_tree: IAggStateTransition;
    op_add_l1_withdrawal_transition_user_state_tree: IAggStateTransition;
    op_add_l1_withdrawal_proof_id: IQProvingJobDataID;
}

interface ICRAggProcessL1WithdrawalAddL1DepositCircuitInput {
    op_process_l1_withdrawal_transition_withdrawal_tree: IAggStateTransitionWithEvents;
    op_process_l1_withdrawal_proof_id: IQProvingJobDataID;
    op_add_l1_deposit_transition_deposit_tree: IAggStateTransitionWithEvents;
    op_add_l1_deposit_proof_id: IQProvingJobDataID;
}

interface IBaseJobWitness {
    q_witness_type: keyof typeof ProvingJobCircuitType;
}
interface IRegisterUserJobWitness extends ICRUserRegistrationCircuitInput, IBaseJobWitness {
    q_witness_type: "RegisterUser";
}

interface IRegisterUserAggregateJobWitness extends IAggStateTransitionInput, IBaseJobWitness {
    q_witness_type: "RegisterUserAggregate";
}

interface IAddL1DepositJobWitness extends ICRAddL1DepositCircuitInput, IBaseJobWitness {
    q_witness_type: "AddL1Deposit";
}

interface IAddL1DepositAggregateJobWitness extends IAggStateTransitionWithEventsInput, IBaseJobWitness {
    q_witness_type: "AddL1DepositAggregate";
}

interface IClaimL1DepositJobWitness extends ICRClaimL1DepositCircuitInput, IBaseJobWitness {
    q_witness_type: "ClaimL1Deposit";
}

interface IClaimL1DepositAggregateJobWitness extends IAggStateTransitionInput, IBaseJobWitness {
    q_witness_type: "ClaimL1DepositAggregate";
}

interface ITransferTokensL2JobWitness extends ICRL2TransferCircuitInput, IBaseJobWitness {
    q_witness_type: "TransferTokensL2";
}

interface ITransferTokensL2AggregateJobWitness extends IAggStateTransitionInput, IBaseJobWitness {
    q_witness_type: "TransferTokensL2Aggregate";
}

interface IAddL1WithdrawalJobWitness extends ICRAddL1WithdrawalCircuitInput, IBaseJobWitness {
    q_witness_type: "AddL1Withdrawal";
}

interface IAddL1WithdrawalAggregateJobWitness extends IAggStateTransitionInput, IBaseJobWitness {
    q_witness_type: "AddL1WithdrawalAggregate";
}

interface IProcessL1WithdrawalJobWitness extends ICRProcessL1WithdrawalCircuitInput, IBaseJobWitness {
    q_witness_type: "ProcessL1Withdrawal";
}

interface IProcessL1WithdrawalAggregateJobWitness extends IAggStateTransitionWithEventsInput, IBaseJobWitness {
    q_witness_type: "ProcessL1WithdrawalAggregate";
}

interface IGenerateRollupStateTransitionProofJobWitness extends ICRBlockStateTransitionCircuitInput, IBaseJobWitness {
    q_witness_type: "GenerateRollupStateTransitionProof";
}

interface IGenerateSigHashIntrospectionProofJobWitness extends ICRSigHashWrapperCircuitInput, IBaseJobWitness {
    q_witness_type: "GenerateSigHashIntrospectionProof";
}

interface IGenerateFinalSigHashProofJobWitness extends ICRSigHashFinalGLCircuitInput, IBaseJobWitness {
    q_witness_type: "GenerateFinalSigHashProof";
}

interface IWrapFinalSigHashProofBLS12381JobWitness extends IQProvingJobDataID, IBaseJobWitness {
    q_witness_type: "WrapFinalSigHashProofBLS12381";
}

interface IAggUserRegisterClaimDepositL2TransferJobWitness
    extends ICRAggUserRegisterClaimDepositL2TransferCircuitInput,
        IBaseJobWitness {
    q_witness_type: "AggUserRegisterClaimDepositL2Transfer";
}

interface IAggAddProcessL1WithdrawalAddL1DepositJobWitness
    extends ICRAggAddProcessL1WithdrawalAddL1DepositCircuitInput,
        IBaseJobWitness {
    q_witness_type: "AggAddProcessL1WithdrawalAddL1Deposit";
}

interface IDummyRegisterUserAggregateJobWitness extends IDummyAggStateTransition, IBaseJobWitness {
    q_witness_type: "DummyRegisterUserAggregate";
}

interface IDummyAddL1DepositAggregateJobWitness extends IDummyAggStateTransitionWithEvents, IBaseJobWitness {
    q_witness_type: "DummyAddL1DepositAggregate";
}

interface IDummyClaimL1DepositAggregateJobWitness extends IDummyAggStateTransition, IBaseJobWitness {
    q_witness_type: "DummyClaimL1DepositAggregate";
}

interface IDummyTransferTokensL2AggregateJobWitness extends IDummyAggStateTransition, IBaseJobWitness {
    q_witness_type: "DummyTransferTokensL2Aggregate";
}

interface IDummyAddL1WithdrawalAggregateJobWitness extends IDummyAggStateTransition, IBaseJobWitness {
    q_witness_type: "DummyAddL1WithdrawalAggregate";
}

interface IDummyProcessL1WithdrawalAggregateJobWitness extends IDummyAggStateTransitionWithEvents, IBaseJobWitness {
    q_witness_type: "DummyProcessL1WithdrawalAggregate";
}

type ICityJobWitness =
    | IRegisterUserJobWitness
    | IRegisterUserAggregateJobWitness
    | IAddL1DepositJobWitness
    | IAddL1DepositAggregateJobWitness
    | IClaimL1DepositJobWitness
    | IClaimL1DepositAggregateJobWitness
    | ITransferTokensL2JobWitness
    | ITransferTokensL2AggregateJobWitness
    | IAddL1WithdrawalJobWitness
    | IAddL1WithdrawalAggregateJobWitness
    | IProcessL1WithdrawalJobWitness
    | IProcessL1WithdrawalAggregateJobWitness
    | IGenerateRollupStateTransitionProofJobWitness
    | IGenerateSigHashIntrospectionProofJobWitness
    | IGenerateFinalSigHashProofJobWitness
    | IWrapFinalSigHashProofBLS12381JobWitness
    | IAggUserRegisterClaimDepositL2TransferJobWitness
    | IAggAddProcessL1WithdrawalAddL1DepositJobWitness
    | IDummyRegisterUserAggregateJobWitness
    | IDummyAddL1DepositAggregateJobWitness
    | IDummyClaimL1DepositAggregateJobWitness
    | IDummyTransferTokensL2AggregateJobWitness
    | IDummyAddL1WithdrawalAggregateJobWitness
    | IDummyProcessL1WithdrawalAggregateJobWitness;

export type {
    IBTCRollupIntrospectionResultDeposit,
    ICRAddL1DepositCircuitInput,
    ICRAddL1WithdrawalCircuitInput,
    ICRClaimL1DepositCircuitInput,
    ICRL2TransferCircuitInput,
    ICRProcessL1WithdrawalCircuitInput,
    ICRUserRegistrationCircuitInput,
    IDummyAggStateTransition,
    IDummyAggStateTransitionWithEvents,
    IAggStateTransition,
    IAggStateTransitionInput,
    IAggStateTransitionWithEvents,
    IAggStateTransitionWithEventsInput,
    ICRAggUserRegisterClaimDepositL2TransferCircuitInput,
    ICRAggUserRegisterClaimDepositL2TransferStateTransition,
    ICRAggAddProcessL1WithdrawalAddL1DepositCircuitInput,
    ICRAggAddProcessL1WithdrawalAddL1DepositStateTransition,
    ICRBlockStateTransitionCircuitInput,
    ICRAggL2TransferAddL1WithdrawalCircuitInput,
    ICRAggProcessL1WithdrawalAddL1DepositCircuitInput,
    ICRSigHashWrapperCircuitInput,
    IBTCRollupIntrospectionFinalizedResult,
    ICRSigHashFinalGLCircuitInput,
    IRegisterUserJobWitness,
    IRegisterUserAggregateJobWitness,
    IAddL1DepositJobWitness,
    IAddL1DepositAggregateJobWitness,
    IClaimL1DepositJobWitness,
    IClaimL1DepositAggregateJobWitness,
    ITransferTokensL2JobWitness,
    ITransferTokensL2AggregateJobWitness,
    IAddL1WithdrawalJobWitness,
    IAddL1WithdrawalAggregateJobWitness,
    IProcessL1WithdrawalJobWitness,
    IProcessL1WithdrawalAggregateJobWitness,
    IGenerateRollupStateTransitionProofJobWitness,
    IGenerateSigHashIntrospectionProofJobWitness,
    IGenerateFinalSigHashProofJobWitness,
    IWrapFinalSigHashProofBLS12381JobWitness,
    IAggUserRegisterClaimDepositL2TransferJobWitness,
    IAggAddProcessL1WithdrawalAddL1DepositJobWitness,
    IDummyRegisterUserAggregateJobWitness,
    IDummyAddL1DepositAggregateJobWitness,
    IDummyClaimL1DepositAggregateJobWitness,
    IDummyTransferTokensL2AggregateJobWitness,
    IDummyAddL1WithdrawalAggregateJobWitness,
    IDummyProcessL1WithdrawalAggregateJobWitness,
    ICityJobWitness,
};
