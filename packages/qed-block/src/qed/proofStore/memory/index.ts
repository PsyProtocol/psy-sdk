/*

pub trait QProofStoreReaderSync {
    fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;
    fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>>;
    fn get_goal_by_job_id(&self, id: QProvingJobDataID) -> anyhow::Result<u32> {
        let counter_id = id.get_sub_group_counter_id();
        let goal_id = counter_id.get_sub_group_counter_goal_id();
        //tracing::info!("goal_id: {:?}", goal_id);
        let goal = self.get_bytes_by_id(goal_id)?;
        Ok(u32::from_le_bytes(goal.try_into().unwrap()))
    }
    fn get_next_jobs_by_job_id(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<QProvingJobDataID>> {
        let counter_id = id.get_sub_group_counter_id();
        let next_jobs_id = counter_id.get_sub_group_counter_goal_next_jobs_id();
        let next_jobs = self.get_bytes_by_id(next_jobs_id)?;
        Ok(bincode::deserialize(&next_jobs)?)
    }
}

pub trait QProofStoreWriterSync {
    fn set_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &mut self,
        id: QProvingJobDataID,
        proof: &ProofWithPublicInputs<C::F, C, D>,
    ) -> anyhow::Result<()>;
    fn set_bytes_by_id(&mut self, id: QProvingJobDataID, data: &[u8]) -> anyhow::Result<()>;

    fn inc_counter_by_id(&mut self, id: QProvingJobDataID) -> anyhow::Result<u32>;
    fn write_next_jobs(
        &mut self,
        jobs: &[QProvingJobDataID],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()>;
    fn write_next_jobs_core(
        &mut self,
        jobs: &[QProvingJobDataID],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        let counter_id = jobs[0].get_sub_group_counter_id();
        let goal_id = counter_id.get_sub_group_counter_goal_id();
        let next_jobs_id = counter_id.get_sub_group_counter_goal_next_jobs_id();
        self.set_bytes_by_id(counter_id, &u32::to_le_bytes(0))?;
        self.set_bytes_by_id(goal_id, &u32::to_le_bytes(jobs.len() as u32))?;
        self.set_bytes_by_id(next_jobs_id, &bincode::serialize(next_jobs)?)?;
        Ok(())
    }

    fn write_multidimensional_jobs(
        &mut self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()>;
    fn write_multidimensional_jobs_core(
        &mut self,
        jobs_levels: &[Vec<QProvingJobDataID>],
        next_jobs: &[QProvingJobDataID],
    ) -> anyhow::Result<()> {
        let job_levels_count = jobs_levels.len();
        for i in 0..job_levels_count {
            let counter_id = jobs_levels[i][0].get_sub_group_counter_id();
            let goal_id = counter_id.get_sub_group_counter_goal_id();
            let next_jobs_id = counter_id.get_sub_group_counter_goal_next_jobs_id();
            self.set_bytes_by_id(counter_id, &u32::to_le_bytes(0))?;
            self.set_bytes_by_id(goal_id, &u32::to_le_bytes(jobs_levels[i].len() as u32))?;
            self.set_bytes_by_id(
                next_jobs_id,
                &bincode::serialize(if i == (job_levels_count - 1) {
                    next_jobs
                } else {
                    &jobs_levels[i + 1]
                })?,
            )?;
        }
        Ok(())
    }
}

*/

import { readU32LEFromBytes } from "../../../../src/utils/byteView";
import { CityProofWithPublicInputs } from "../../commonTypes";
import { IQProvingJobDataID, serializeJobIdHex } from "../../job/id";
import {
    getJobSubGroupCounterGoalId,
    getJobSubGroupCounterId,
    getSubGroupCounterGoalNextJobsId,
} from "../../job/idHelpers";
import { IQProofStore } from "../types";
import { deserializeJobIdArray, serializeJobIdArray } from "../../job/idBincode";

class SimpleProofStoreMemory implements IQProofStore {
    data: Record<string, Uint8Array> = {};

    private getDataByJobId(id: IQProvingJobDataID): Uint8Array {
        const key = serializeJobIdHex(id);
        if (Object.hasOwnProperty.call(this.data, key)) {
            const value = this.data[key];
            if (value instanceof Uint8Array) {
                return value;
            }
        }
        throw new Error(`SimpleProofStoreMemory: No data found for id: ${key}`);
    }

    get_proof_by_id(id: IQProvingJobDataID): CityProofWithPublicInputs {
        throw new Error("Method not implemented.");
    }
    set_proof_by_id(id: IQProvingJobDataID, proof: CityProofWithPublicInputs): void {
        throw new Error("Method not implemented.");
    }

    get_bytes_by_id(id: IQProvingJobDataID): Uint8Array {
        return this.getDataByJobId(id);
    }
    get_goal_by_job_id(id: IQProvingJobDataID): number {
        const counter_id = getJobSubGroupCounterId(id);
        const goal_id = getJobSubGroupCounterGoalId(counter_id);
        const bytes = this.getDataByJobId(goal_id);
        if (bytes.length !== 4) {
            throw new Error(`SimpleProofStoreMemory: Invalid goal bytes length: ${bytes.length}`);
        }
        return readU32LEFromBytes(bytes);
    }
    get_next_jobs_by_job_id(id: IQProvingJobDataID): IQProvingJobDataID[] {
        const counter_id = getJobSubGroupCounterId(id);
        const next_jobs_id = getSubGroupCounterGoalNextJobsId(counter_id);
        const bytes = this.getDataByJobId(next_jobs_id);
        return deserializeJobIdArray(bytes);
    }
    set_bytes_by_id(id: IQProvingJobDataID, data: Uint8Array): void {
        const key = serializeJobIdHex(id);
        this.data[key] = data;
    }
    inc_counter_by_id(id: IQProvingJobDataID): number {
        const key = serializeJobIdHex(id);
        const bytes = this.getDataByJobId(id);
        if (bytes.length !== 4) {
            throw new Error(`SimpleProofStoreMemory: Invalid counter bytes length: ${bytes.length}`);
        }
        const value = readU32LEFromBytes(bytes);
        const newValue = value + 1;
        this.data[key] = new Uint8Array(new Uint32Array([newValue]).buffer);
        return newValue;
    }
    write_next_jobs_core(jobs: IQProvingJobDataID[], next_jobs: IQProvingJobDataID[]): void {
        const counter_id = getJobSubGroupCounterId(jobs[0]);
        const goal_id = getJobSubGroupCounterGoalId(counter_id);
        const next_jobs_id = getSubGroupCounterGoalNextJobsId(counter_id);
        this.set_bytes_by_id(counter_id, new Uint8Array([0, 0, 0, 0]));
        this.set_bytes_by_id(goal_id, new Uint8Array(new Uint32Array([jobs.length]).buffer));
        this.set_bytes_by_id(next_jobs_id, serializeJobIdArray(next_jobs));
    }
    write_multidimensional_jobs_core(jobs: IQProvingJobDataID[][], next_jobs: IQProvingJobDataID[]): void {
        let job_levels_count = jobs.length;
        for (let i = 0; i < job_levels_count; i++) {
            const counter_id = getJobSubGroupCounterId(jobs[i][0]);
            const goal_id = getJobSubGroupCounterGoalId(counter_id);
            const next_jobs_id = getSubGroupCounterGoalNextJobsId(counter_id);
            this.set_bytes_by_id(counter_id, new Uint8Array([0, 0, 0, 0]));
            this.set_bytes_by_id(goal_id, new Uint8Array(new Uint32Array([jobs[i].length]).buffer));
            this.set_bytes_by_id(
                next_jobs_id,
                serializeJobIdArray(i === job_levels_count - 1 ? next_jobs : jobs[i + 1])
            );
        }
    }
    write_next_jobs(jobs: IQProvingJobDataID[], next_jobs: IQProvingJobDataID[]): void {
        return this.write_next_jobs_core(jobs, next_jobs);
    }
    write_multidimensional_jobs(jobs_levels: IQProvingJobDataID[][], next_jobs: IQProvingJobDataID[]): void {
        return this.write_multidimensional_jobs_core(jobs_levels, next_jobs);
    }
}

export { SimpleProofStoreMemory };
