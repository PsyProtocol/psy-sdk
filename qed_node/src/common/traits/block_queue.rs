use plonky2::hash::hash_types::RichField;
use qed_core::data::{base_types::hash256::Hash256, qhashout::QHashOut};



pub trait CoordinatorBlockAPIInputQueueImm<F: RichField> {
    fn add_user_registration_request_imm(&self, request_id: Hash256, public_key: QHashOut<F>, fingerprint: QHashOut<F>, public_key_param: QHashOut<F>) -> anyhow::Result<()>;
    fn add_contract_deploy_request_imm(&self, request_id: Hash256, public_key: QHashOut<F>) -> anyhow::Result<()>;
}