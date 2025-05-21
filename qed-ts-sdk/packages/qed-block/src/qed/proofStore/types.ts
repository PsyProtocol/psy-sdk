import { CityProofWithPublicInputs } from "../commonTypes";
import { IQProvingJobDataID } from "../job/id";

interface IQProofStoreReaderSync {
    get_proof_by_id(id: IQProvingJobDataID): CityProofWithPublicInputs;
    get_bytes_by_id(id: IQProvingJobDataID): Uint8Array;
    get_goal_by_job_id(id: IQProvingJobDataID): number;
    get_next_jobs_by_job_id(id: IQProvingJobDataID): IQProvingJobDataID[];
}
interface IQProofStoreWriterSync {
    set_proof_by_id(id: IQProvingJobDataID, proof: CityProofWithPublicInputs): void;
    set_bytes_by_id(id: IQProvingJobDataID, data: Uint8Array): void;
    inc_counter_by_id(id: IQProvingJobDataID): number;
    write_next_jobs(jobs: IQProvingJobDataID[], next_jobs: IQProvingJobDataID[]): void;
    write_multidimensional_jobs(jobs_levels: IQProvingJobDataID[][], next_jobs: IQProvingJobDataID[]): void;
}
interface IQProofStore extends IQProofStoreReaderSync, IQProofStoreWriterSync {}

export type { IQProofStoreReaderSync, IQProofStoreWriterSync, IQProofStore };
