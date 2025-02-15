use async_trait::async_trait;
use plonky2::{field::types::PrimeField64, plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs}};
use qed_core::job::{id::QProvingJobDataID, traits::{QProofStore, QProofStoreWriterSyncImm}};
use qed_data::{guta::api::SubmitUserEndCapProofAPIInput, qsync::coordinator::QEDCheckpointSyncInfoCompact};
use qed_store::{config::store_config::{QCheckpointSyncInfoCompact, QEDFelt}, node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsync}};
type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

type QProof = ProofWithPublicInputs<F,C,D>;
type QUserEndCapAPIInput = SubmitUserEndCapProofAPIInput<F,C,D>;

#[async_trait]
pub trait EdgeContext{
    async fn get_checkpoint_u64<S: QEDRealmStoreReaderAsync<F> + Sync>(store: &S)->anyhow::Result<u64> {
        Ok(store.get_latest_l2_block_state().await?.checkpoint_id)
    }
    fn get_rpc_node_id(&self)->u32;
    async fn enqueue_user_end_cap_job<PS: QProofStoreWriterSyncImm + Sync>(&self, ps: &PS, job: QProvingJobDataID) -> anyhow::Result<()>;
    async fn enqueue_user_update<S: QEDRealmStoreReaderAsync<F> + Sync, PS: QProofStoreWriterSyncImm + Sync>(&self, store: &S, ps: &PS, validated_input: QUserEndCapAPIInput) -> anyhow::Result<()> {

        let checkpoint_id = Self::get_checkpoint_u64(store).await?;

        let job_id =QProvingJobDataID::end_cap_proof(self.get_rpc_node_id(), checkpoint_id, validated_input.input.core.state_transition.user_id.to_canonical_u64() as u32);

        

        ps.set_proof_by_id_imm(job_id.get_output_id(), &validated_input.proof)?;
        ps.set_bytes_by_id_imm(job_id.get_input_witness_id(), &bincode::serialize(&validated_input.input)?)?;

        Ok(())
    }
}



#[async_trait]
pub trait CoordinatorAPIStateHelperImm: EdgeContext {
    async fn recv_checkpoint_sync_base<S: QEDRealmStoreReaderAsync<F>+QEDRealmStoreWriterAsync<F> + Sync, PS: QProofStoreWriterSyncImm + Sync>(&self, store: &S, ps: &PS, checkpoint_sync_info: QCheckpointSyncInfoCompact) -> anyhow::Result<()>{
        //store.

        Ok(())
        
    }
    async fn recv_checkpoint_sync<S: QEDRealmStoreReaderAsync<F>+QEDRealmStoreWriterAsync<F> + Sync, PS: QProofStoreWriterSyncImm + Sync>(&self, store: &S, ps: &PS, checkpoint_sync_info: QCheckpointSyncInfoCompact) -> anyhow::Result<()>;
    async fn recv_checkpoint_sync_old(&self, checkpoint_sync_info: QCheckpointSyncInfoCompact) -> anyhow::Result<()>;

}