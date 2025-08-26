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
use qed_data::config::store_config::QEDHasher;
use qed_prover::local::args::ContractCallArgs;
use qed_prover::session::WalletSession;
use scheduled_thread_pool::ScheduledThreadPool;
use std::sync::Arc;
use std::time::Instant;
use tracing::log::info;

pub async fn run(args: StressTestArgs) -> Result<()> {
    let rpc_config = load_rpc_config(&args.config)?;
    let multicall = Multicast::new(rpc_config)?;
    let pool = ScheduledThreadPool::new(num_cpus::get());
    let handle = pool.execute(move || loop {
        info!("🎯 Registering batch user - User count: {}", 1000);
        multicall.register_batch_user(1000).unwrap();
    });

    tokio::signal::ctrl_c().await?;
    handle.cancel();
    info!("🛑 Stopping stress test...");
    Ok(())
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

    pub async fn exec_contract_call(
        &self,
        pk_hash: QHashOut<GoldilocksField>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<()> {
        self.wallet_session
            .write()
            .exec_contract_call(pk_hash, contract_call_args)
    }

    pub fn register_batch_user(&self, user_count: u64) -> Result<()> {
        info!(
            "register_batch_user: Registering batch user - User count: {}",
            user_count
        );
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
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!(
                "register batch user timeout waiting for checkpoint"
            ));
        }
        for pk in &user_pk {
            let pk_info = self.wallet_session.read().get_zk_public_key(pk.clone())?;
            let pk_hash = pk_info.qfhash::<QEDHasher>();
            let user_id = self
                .wallet_session
                .read()
                .st_provider
                .get_user_id(pk_hash)?;
            info!(
                "register_batch_user: User_id: {}, pk_hash: {}",
                user_id, pk_hash
            );
        }
        info!("Registered batch user");
        Ok(())
    }
}
