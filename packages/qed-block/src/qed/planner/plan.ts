import {
    blockAggStatePart1InputWitnessJobId,
    blockAggStatePart2InputWitnessJobId,
    blockStateTransitionInputWitnessJobId,
    getBlockAggregateJobsGroupJobId,
    notifyBlockCompleteJobId,
    sighashFinalInputWitnessJobId,
    sighashIntrospectionInputWitnessJobId,
    wrapSighashFinalBls3812InputWitnessJobId,
} from "../job/idHelpers";
import { IQProofStore } from "../proofStore/types";
import { ICityOpJobIds } from "./transition";

function planJobs(
    proof_store: IQProofStore,
    block_op_job_ids: ICityOpJobIds,
    num_input_witnesses: number,
    checkpoint_id: number
) {
    let root_state_transition = blockStateTransitionInputWitnessJobId(checkpoint_id);

    let agg_jobs_for_inputs = Array.from({ length: num_input_witnesses }, (v, i) =>
        getBlockAggregateJobsGroupJobId(checkpoint_id, 1, i)
    );

    proof_store.write_next_jobs(agg_jobs_for_inputs, [notifyBlockCompleteJobId(checkpoint_id)]);

    let per_input_jobs = Array.from({ length: num_input_witnesses }, (v, i) => {
        return [
            wrapSighashFinalBls3812InputWitnessJobId(checkpoint_id, i),
            sighashFinalInputWitnessJobId(checkpoint_id, i),
            sighashIntrospectionInputWitnessJobId(checkpoint_id, i),
        ];
    });

    for (let i = 0; i < per_input_jobs.length; i++) {
        proof_store.write_next_jobs([per_input_jobs[i][0]], [agg_jobs_for_inputs[i]]);
        proof_store.write_next_jobs([per_input_jobs[i][1]], [per_input_jobs[i][0]]);
    }
    let agg_state_and_introspections_group_id = 5;
    let agg_state_root_id = getBlockAggregateJobsGroupJobId(checkpoint_id, agg_state_and_introspections_group_id, 0);
    let agg_all_introspections_ids = getBlockAggregateJobsGroupJobId(
        checkpoint_id,
        agg_state_and_introspections_group_id,
        1
    );
    let introspection_jobs = per_input_jobs.map((x) => x[2]);
    proof_store.write_next_jobs(introspection_jobs, [agg_all_introspections_ids]);
    let final_input_witness_jobs = per_input_jobs.map((x) => x[1]);
    proof_store.write_next_jobs([agg_state_root_id, agg_all_introspections_ids], final_input_witness_jobs);

    proof_store.write_next_jobs([root_state_transition], [agg_state_root_id]);

    let op_agg_group_parts_common_id = 6;

    let state_part_1_common_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_parts_common_id, 0);
    let state_part_2_common_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_parts_common_id, 1);

    let state_part_1_id = blockAggStatePart1InputWitnessJobId(checkpoint_id);
    let state_part_2_id = blockAggStatePart2InputWitnessJobId(checkpoint_id);

    proof_store.write_next_jobs([state_part_1_common_id, state_part_2_common_id], [root_state_transition]);

    proof_store.write_next_jobs([state_part_1_id], [state_part_1_common_id]);
    proof_store.write_next_jobs([state_part_2_id], [state_part_2_common_id]);

    let op_agg_group_part_1_id = 11;
    let register_users_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_1_id, 0);
    let claim_deposits_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_1_id, 1);
    let transfer_tokens_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_1_id, 2);

    proof_store.write_next_jobs(
        [register_users_agg_job_id, claim_deposits_agg_job_id, transfer_tokens_agg_job_id],
        [state_part_1_id]
    );

    let op_agg_group_part_2_id = 12;
    let add_withdrawals_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_2_id, 0);
    let process_withdrawals_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_2_id, 1);
    let add_deposits_agg_job_id = getBlockAggregateJobsGroupJobId(checkpoint_id, op_agg_group_part_2_id, 2);

    proof_store.write_next_jobs(
        [add_withdrawals_agg_job_id, process_withdrawals_agg_job_id, add_deposits_agg_job_id],
        [state_part_2_id]
    );

    proof_store.write_multidimensional_jobs(block_op_job_ids.register_user_job_ids, [register_users_agg_job_id]);
    proof_store.write_multidimensional_jobs(block_op_job_ids.claim_deposit_job_ids, [claim_deposits_agg_job_id]);
    proof_store.write_multidimensional_jobs(block_op_job_ids.token_transfer_job_ids, [transfer_tokens_agg_job_id]);

    proof_store.write_multidimensional_jobs(block_op_job_ids.add_withdrawal_job_ids, [add_withdrawals_agg_job_id]);
    proof_store.write_multidimensional_jobs(block_op_job_ids.process_withdrawal_job_ids, [
        process_withdrawals_agg_job_id,
    ]);
    proof_store.write_multidimensional_jobs(block_op_job_ids.add_deposit_job_ids, [add_deposits_agg_job_id]);

    let leaf_jobs = introspection_jobs
        .concat(block_op_job_ids.register_user_job_ids[0])
        .concat(block_op_job_ids.claim_deposit_job_ids[0])
        .concat(block_op_job_ids.token_transfer_job_ids[0])
        .concat(block_op_job_ids.add_withdrawal_job_ids[0])
        .concat(block_op_job_ids.process_withdrawal_job_ids[0])
        .concat(block_op_job_ids.add_deposit_job_ids[0]);

    return leaf_jobs;
}

export { planJobs };
