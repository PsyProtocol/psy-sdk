import { IQJobWithDependenciesSerialized } from "../bench";
import { IQProvingJobDataID, ProvingJobCircuitType, serializeJobId, serializeJobIdHex } from "../job/id";
import { getJobOutputId, getJobTreeParentProofInputId, newCoreOpWitnessJobId, newProofJobId } from "../job/idHelpers";
import { createBinaryTreePlanner } from "./binaryTree";

function getDummyWalkTreeProverIds<T>(
    leaves: IQProvingJobDataID[],
    dummyId: IQProvingJobDataID,
    visitJob: (id: IQProvingJobDataID, children: T[]) => T
): T[][] {
    if (leaves.length === 0) {
        return [[visitJob(dummyId, [])]];
    } else {
        const leavesLen = leaves.length;
        const levels = createBinaryTreePlanner(leavesLen).levels;
        const jobIds = [leaves];
        const jobs = [leaves.map((id) => visitJob(id, []))];

        for (const levelNodes of levels) {
            const levelJobIds: IQProvingJobDataID[] = [];
            const levelJobs: T[] = [];
            for (const node of levelNodes) {
                const leftProofId = getJobOutputId(jobIds[node.left_job.level][node.left_job.index]);
                const selfWitnessId = getJobTreeParentProofInputId(leftProofId);
                const job = visitJob(selfWitnessId, [
                    jobs[node.left_job.level][node.left_job.index],
                    jobs[node.right_job.level][node.right_job.index],
                ]);
                levelJobIds.push(selfWitnessId);
                levelJobs.push(job);
            }
            jobIds.push(levelJobIds);
            jobs.push(levelJobs);
        }
        return jobs;
    }
}
function getDummyTreeProverIds(leaves: IQProvingJobDataID[], dummyId: IQProvingJobDataID): IQProvingJobDataID[][] {
    if (leaves.length === 0) {
        return [[dummyId]];
    } else {
        const leavesLen = leaves.length;
        const levels = createBinaryTreePlanner(leavesLen).levels;
        const jobIds = [leaves];

        for (const levelNodes of levels) {
            const levelJobIds: IQProvingJobDataID[] = [];
            for (const node of levelNodes) {
                const leftProofId = getJobOutputId(jobIds[node.left_job.level][node.left_job.index]);
                const selfWitnessId = getJobTreeParentProofInputId(leftProofId);
                levelJobIds.push(selfWitnessId);
            }
            jobIds.push(levelJobIds);
        }
        return jobIds;
    }
}
function getDummyTreeProverIdsOpCircuit(
    circuitType: ProvingJobCircuitType,
    dummyType: ProvingJobCircuitType,
    checkpointId: number,
    leafCount: number
): IQProvingJobDataID[][] {
    const dummyId = newProofJobId(checkpointId, dummyType, 0xdd, 0, 0);
    const leaves: IQProvingJobDataID[] = [];
    for (let i = 0; i < leafCount; i++) {
        leaves.push(newCoreOpWitnessJobId(circuitType, checkpointId, i));
    }
    return getDummyTreeProverIds(leaves, dummyId);
}
function getDummyTreeOpCircuitJobWithDependencies(
    circuitType: ProvingJobCircuitType,
    dummyType: ProvingJobCircuitType,
    checkpointId: number,
    leafCount: number
): IQJobWithDependenciesSerialized {
    const dummyId = newProofJobId(checkpointId, dummyType, 0xdd, 0, 0);
    const leaves: IQProvingJobDataID[] = [];
    for (let i = 0; i < leafCount; i++) {
        leaves.push(newCoreOpWitnessJobId(circuitType, checkpointId, i));
    }
    const visitJob = (id: IQProvingJobDataID, children: IQJobWithDependenciesSerialized[]) => {
        const result: IQJobWithDependenciesSerialized = {
            id: serializeJobIdHex(id),
            dependencies: children.concat([]),
        };
        return result;
    };
    const result = getDummyWalkTreeProverIds<IQJobWithDependenciesSerialized>(leaves, dummyId, visitJob);
    return result.pop()![0];
}

export {
    getDummyTreeProverIdsOpCircuit,
    getDummyTreeProverIds,
    getDummyWalkTreeProverIds,
    getDummyTreeOpCircuitJobWithDependencies,
};
