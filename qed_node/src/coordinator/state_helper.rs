use async_trait::async_trait;
use plonky2::{
    field::types::{Field, PrimeField64},
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_core::job::{
    id::QProvingJobDataID,
    traits::QProofStoreWriterSyncImm,
};
use psy_crypto::common::user_id::get_user_id_from_registration_id;
use psy_data::{
    api::coordinator::register_user::QEDAPIRegisterUserRequestForUserId, guta::api::SubmitUserEndCapProofAPIInput
};
use psy_data::config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher};
use qed_store::node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsyncImm};
type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

type QProof = ProofWithPublicInputs<F, C, D>;
type QUserEndCapAPIInput = SubmitUserEndCapProofAPIInput<F, C, D>;

#[async_trait]
pub trait EdgeContext {
    async fn get_checkpoint_u64<S: QEDRealmStoreReaderAsync<F> + Sync>(
        store: &S,
    ) -> anyhow::Result<u64> {
        Ok(store.get_latest_l2_block_state().await?.checkpoint_id)
    }
    fn get_node_id(&self) -> u32;
    async fn enqueue_user_end_cap_job<PS: QProofStoreWriterSyncImm + Sync>(
        &self,
        ps: &PS,
        job: QProvingJobDataID,
    ) -> anyhow::Result<()>;
    async fn enqueue_user_registrations<PS: QProofStoreWriterSyncImm + Sync>(&self, ps: &PS, user_registrations: Vec<QEDAPIRegisterUserRequestForUserId<F>>) -> anyhow::Result<()>;
    async fn enqueue_user_update<
        S: QEDRealmStoreReaderAsync<F> + Sync,
        PS: QProofStoreWriterSyncImm + Sync,
    >(
        &self,
        store: &S,
        ps: &PS,
        validated_input: QUserEndCapAPIInput,
    ) -> anyhow::Result<()> {
        let checkpoint_id = Self::get_checkpoint_u64(store).await?;

        let job_id = QProvingJobDataID::end_cap_proof(
            checkpoint_id,
            0,
            self.get_node_id(),
            validated_input
                .input
                .core
                .state_transition
                .user_id
                .to_canonical_u64() as u32,
        );

        ps.set_proof_by_id_imm(job_id.get_output_id(), &validated_input.proof)?;
        ps.set_bytes_by_id_imm(
            job_id.get_input_witness_id(),
            &bincode::serialize(&validated_input.input)?,
        )?;

        Ok(())
    }
}

#[async_trait]
pub trait CoordinatorAPIStateHelperImm: EdgeContext {
    fn realm_contains_user_id(&self, user_id: u64) -> bool;
    async fn recv_checkpoint_sync_base<
        S: QEDRealmStoreReaderAsync<F> + QEDRealmStoreWriterAsyncImm<F> + Sync,
        PS: QProofStoreWriterSyncImm + Sync,
    >(
        &self,
        store: &S,
        ps: &PS,
        checkpoint_sync_info: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()> {
        //store.
        let start_registration_id = checkpoint_sync_info.l2_block_state.next_user_id-checkpoint_sync_info.registered_users.len() as u64;
        let mut new_good_users: Vec<QEDAPIRegisterUserRequestForUserId<F>> = Vec::new();

        for (registration_id, reg) in (start_registration_id..checkpoint_sync_info.l2_block_state.next_user_id).zip(checkpoint_sync_info.registered_users.iter()) {
           let user_id = get_user_id_from_registration_id(registration_id);
            if self.realm_contains_user_id(user_id) {
                new_good_users.push(QEDAPIRegisterUserRequestForUserId{
                    user_id: F::from_noncanonical_u64(user_id),
                    fingerprint: reg.fingerprint,
                    public_key_param: reg.public_key_param,
                });
            }
        }
        self.enqueue_user_registrations(ps, new_good_users).await?;
        store.injest_checkpoint_sync_data_imm(checkpoint_sync_info.to_sync_info::<QEDHasher>()).await?;



        Ok(())
    }
    async fn recv_checkpoint_sync<
        S: QEDRealmStoreReaderAsync<F> + QEDRealmStoreWriterAsyncImm<F> + Sync,
        PS: QProofStoreWriterSyncImm + Sync,
    >(
        &self,
        store: &S,
        ps: &PS,
        checkpoint_sync_info: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()>;
    async fn recv_checkpoint_sync_old(
        &self,
        checkpoint_sync_info: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()>;
}
