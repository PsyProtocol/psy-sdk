use crate::subcommand::{
    stress_test::{load_rpc_config, wait_for_new_block, RpcConfig},
    StressTestArgs,
};
use anyhow::Result;
use num_cpus;
use parking_lot::RwLock;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::config::store_config::QEDHasher;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::local::args::ContractCallArgs;
use qed_prover::session::WalletSession;
use scheduled_thread_pool::ScheduledThreadPool;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tracing::log::{error, info};

pub async fn run(args: StressTestArgs) -> Result<()> {
    let rpc_config = load_rpc_config(&args.config)?;
    let multicall = Multicast::new(rpc_config)?;
    let pool = ScheduledThreadPool::new(num_cpus::get());
    // let mut handlers = vec![];
    let handle = pool.execute(move || {
        info!("🎯 Registering batch user - User count: {}", args.concurrent_tasks);
        let user_info = multicall.register_batch_user(args.concurrent_tasks as u64).unwrap();
        multicall.batch_flow(user_info).unwrap();
    });

    tokio::signal::ctrl_c().await?;
    handle.cancel();
    info!("🛑 Stopping stress test...");
    Ok(())
}

struct UserInfo {
    user_id: u64,
    pk: QHashOut<GoldilocksField>,
    pub_key: ZKPublicKeyInfo<GoldilocksField>,
}

pub struct Multicast {
    wallet_session: Arc<RwLock<WalletSession>>,
    pool: ScheduledThreadPool,
}

impl Multicast {
    pub fn new(rpc_config: RpcConfig) -> Result<Self> {
        let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config)?));
        let pool = ScheduledThreadPool::new(num_cpus::get());
        Ok(Self {
            wallet_session,
            pool,
        })
    }

    pub fn exec_contract_call(
        &self,
        pk_hash: QHashOut<GoldilocksField>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<()> {
        self.wallet_session
            .write()
            .exec_contract_call(pk_hash, contract_call_args)
    }

    pub fn register_batch_user(&self, user_count: u64) -> Result<Vec<UserInfo>> {
        let user_pk = (0..user_count)
            .map(|_| QHashOut::<GoldilocksField>::rand())
            .collect::<Vec<_>>();
        let start = Instant::now();
        for pk in &user_pk {
            self.wallet_session.write().register_user(pk.clone())?;
        }
        let duration = start.elapsed().as_millis() as u64;
        info!(
            "register_batch_user: Register batch user duration: {} ms",
            duration
        );

        let now = Instant::now();
        let user_info = loop {
            let mut user_info = Vec::new();
            if let Err(err) = wait_for_new_block(&self.wallet_session.read().st_provider, 4) {
                error!("register_batch_user: Wait for new block error: {}", err);
                continue
            }
            for pk in &user_pk {
                match self.wallet_session.read().get_zk_public_key(pk.clone()) {
                    Ok(pk_info) => {
                        let pk_hash = pk_info.qfhash::<QEDHasher>();
                        match self.wallet_session.read().st_provider.get_user_id(pk_hash) {
                            Ok(user_id) => {
                                user_info.push(UserInfo {
                                    user_id,
                                    pk: pk.clone(),
                                    pub_key: pk_info,
                                });
                                info!(
                                    "register_batch_user: User_id: {}, pk_hash: {}",
                                    user_id, pk_hash
                                );
                            }
                            Err(err) => error!("register_batch_user: Get user id error: {}", err),
                        }
                    }
                    Err(err) => error!("register_batch_user: Get zk public key error: {}", err),
                }
            }
            if user_info.len() == user_pk.len() {
                break user_info;
            }
            info!("register_batch_user: Waiting for new block... user_info length: {}, user_pk length: {}", user_info.len(), user_pk.len());
        };
        let duration = now.elapsed().as_millis() as u64;
        info!(
            "register_batch_user: Register batch user duration: {} ms, user_info length: {}",
            duration,
            user_info.len()
        );
        Ok(user_info)
    }

    pub fn batch_flow(&self, user_info: Vec<UserInfo>) -> Result<()> {
        let mut contract_call_args = vec![];
        let from_user_id = user_info[0].user_id;
        let mint_amount = 100000u64;
        contract_call_args.push(ContractCallArgs {
            contract_id: 0,
            method_name: "simple_mint".to_string(),
            inputs: vec![mint_amount],
        });
        for i in 1..user_info.len() {
            let to_user_id = user_info[i].user_id;
            let transfer_amount = 2u64;
            contract_call_args.push(ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![to_user_id, transfer_amount],
            });
        }
        let start = Instant::now();
        self.exec_contract_call(user_info[0].pk, contract_call_args)?;
        let duration = start.elapsed().as_millis() as u64;
        info!("batch_flow: Batch flow duration: {} ms", duration);
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("mint timeout waiting for checkpoint"));
        }

        for i in 1..user_info.len() {
            let claim_contract_call_args = vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_claim".to_string(),
                inputs: vec![mint_amount],
            }];
            info!(
                "Start to execute claim contract call for user {}",
                user_info[i].pk
            );
            let start = Instant::now();
            self.exec_contract_call(user_info[i].pk.clone(), claim_contract_call_args)
                .map_err(|err| anyhow::format_err!("exec_contract_call: {}", err))?;
            let duration = start.elapsed().as_millis() as u64;
            info!(
                "🔄 Task {} - Claim contract call duration: {} ms",
                i, duration
            );
            info!(
                "End to execute claim contract call for user {}",
                user_info[i].pk
            );
        }

        if !wait_for_new_block(&self.wallet_session.read().st_provider, 1)? {
            return Err(anyhow::format_err!("claim timeout waiting for checkpoint"));
        }

        Ok(())
    }
}
