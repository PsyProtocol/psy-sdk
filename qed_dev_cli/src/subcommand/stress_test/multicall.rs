use std::fs;
use std::path::Path;
use std::str::FromStr;
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
use qed_prover::session::{gen_contract_deploy_and_circuits_for_functions, WalletSession};
use scheduled_thread_pool::ScheduledThreadPool;
use std::sync::Arc;
use std::time::Instant;
use tracing::log::{error, info};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::config::network_constants::MAX_CONTRACT_STATE_TREE_HEIGHT;
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_prover::local::provider::{QUserRpcProvider, RpcProvider};
use qed_prover::local::request::QDeployContractRPCRequest;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use qed_data::config::store_config::{C, D};

const USER0_PRIVATE_KEY: &str = "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a";
const USER0_PUBLIC_KEY: &str = "6ee6d9596a34a5de293cb550d5d100d00b30487245777018677cc803345633c5";
const USER0_SECP_ZK_PUBLIC_KEY: &str = "49deab842acf3d26236419d4fce1b2cb01081aef55d4ef0e566f980e3890cf2f";

const USER1_PRIVATE_KEY: &str = "f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d";
const USER1_PUBLIC_KEY: &str = "0aa313de0677ed55f51cca7094b519d53d661f131f481a03e12e45f0f3389f12";
const USER1_SECP_ZK_PUBLIC_KEY: &str = "ac85e11f5c8a53241502c4519567aa3f02d30b1639fc49bb94c1d61335197e1a";

pub async fn run(args: StressTestArgs) -> Result<()> {
    let rpc_config = load_rpc_config(&args.config)?;
    let multicall = Multicast::new(rpc_config)?;
    let pool = ScheduledThreadPool::new(num_cpus::get());
    // let mut handlers = vec![];
    let handle = pool.execute(move || {
        for repeat in 0..args.repeat {
            info!("🎯 Registering batch user - User count: {}, repeat: {}", args.concurrent_tasks, repeat);
            if args.only_user {
                let _ = multicall.register_batch_user(args.concurrent_tasks as u64).unwrap();
            }
            if args.only_flow {
                let user_info = multicall.register_batch_user(args.concurrent_tasks as u64).unwrap();
                multicall.batch_flow(user_info).unwrap();
            }
            if args.only_multi_transfer {
                multicall.batch_multi_transfer(args.concurrent_tasks as u64).unwrap();
            }
            if args.only_multi_user_transfer {
                multicall.multi_user_transfer(args.concurrent_tasks as u64).unwrap();
            }
            if args.only_mint {
                multicall.multi_user_mint(args.concurrent_tasks as u64).unwrap();
            }
            if args.only_deploy_contract {
                multicall.deploy_contract(args.concurrent_tasks as u64, args.contract_path.clone()).unwrap();
            }
        }
    });

    tokio::signal::ctrl_c().await?;
    handle.cancel();
    info!("🛑 Stopping stress test...");
    Ok(())
}

struct UserInfo {
    user_id: u64,
    pk: QHashOut<GoldilocksField>,
    pub_key: QHashOut<GoldilocksField>,
}

pub struct Multicast {
    wallet_session: Arc<RwLock<WalletSession>>,
    pool: ScheduledThreadPool,
    rpc_config: RpcConfig,
}

impl Multicast {
    pub fn new(rpc_config: RpcConfig) -> Result<Self> {
        let wallet_session = Arc::new(RwLock::new(WalletSession::new(&rpc_config)?));
        let pool = ScheduledThreadPool::new(num_cpus::get());
        Ok(Self {
            wallet_session,
            pool,
            rpc_config,
        })
    }

    pub fn exec_contract_call(
        &self,
        public_key: QHashOut<GoldilocksField>,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> Result<()> {
        self.wallet_session
            .write()
            .exec_contract_call(public_key, contract_call_args)
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
                let ret = {self.wallet_session.read().get_secp_public_key(pk.clone())};
                match ret {
                    Ok(pk_info) => {
                        let public_key = pk_info.qfhash::<QEDHasher>();
                        let ret = {self.wallet_session.read().st_provider.get_user_id(public_key)};
                        match ret {
                            Ok(user_id) => {
                                user_info.push(UserInfo {
                                    user_id,
                                    pk: pk.clone(),
                                    pub_key: public_key,
                                });
                                info!("register_batch_user: User_id: {}, pk_hash: {}",user_id, public_key);
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
        let mint_amount = 250000000000u64;
        let transfer_amount = 50000000000u64;
        let (from_user_id, public_key0) = self.init_user0(mint_amount * 2 * user_info.len() as u64)?;
        for i in 0..user_info.len() {
            let to_user_id = user_info[i].user_id;
            contract_call_args.push(ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![to_user_id, transfer_amount],
            });
            {self.wallet_session.write().add_user(user_info[i].pk.clone())?;}
        }

        let count = 10;
        for i in 0..count {
            let to_user_id = user_info[0].user_id;
            contract_call_args.push(ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![to_user_id, 1],
            });
        }

        let start = Instant::now();
        self.exec_contract_call(public_key0, contract_call_args)?;
        let duration = start.elapsed().as_millis() as u64;
        info!("batch_flow: Batch transfer flow duration: {} ms", duration);
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 4)? {
            return Err(anyhow::format_err!("mint timeout waiting for checkpoint"));
        }
        
        info!("batch_flow: Start to execute claim contract call");
        for i in 0..user_info.len() {
            let claim_contract_call_args = vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_claim".to_string(),
                inputs: vec![from_user_id],
            }];
            info!(
                "Start to execute claim contract call for user {}",
                user_info[i].pub_key,
            );
            let start = Instant::now();
            for _ in 0..3 {
                if let Err(err) = self.exec_contract_call(user_info[i].pub_key, claim_contract_call_args.clone())
                    .map_err(|err| anyhow::format_err!("exec_contract_call: {}", err)){
                    error!("❌ Task {} - Claim contract call error: {}",i, err);
                    continue;
                }
                break;
             }

            let duration = start.elapsed().as_millis() as u64;
            info!("🔄 Task {} - Claim contract call duration: {} ms",i, duration);
            info!("End to execute claim contract call for user {}", user_info[i].pub_key);
        }

        info!("batch_flow: claim contract call finished, start to wait for checkpoint");
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 1)? {
            return Err(anyhow::format_err!("claim timeout waiting for checkpoint"));
        }

        info!("batch_flow: end call");
        Ok(())
    }

    pub fn batch_multi_transfer(&self, transfer_count: u64) -> Result<()> {
        let user_info = self.register_batch_user(3)?;
        let mut contract_call_args = vec![];
        let mint_amount = 250000000000u64;
        let transfer_amount = 50000000000u64;
        let (from_user_id, public_key0) = self.init_user0(mint_amount * transfer_count)?;
        for i in 0..user_info.len() {
            for _ in 0..transfer_count {
                let to_user_id = user_info[i].user_id;
                contract_call_args.push(ContractCallArgs {
                    contract_id: 0,
                    method_name: "simple_transfer".to_string(),
                    inputs: vec![to_user_id, transfer_amount],
                });
            }
            {self.wallet_session.write().add_user(user_info[i].pk.clone())?;}
        }
        let start = Instant::now();
        self.exec_contract_call(public_key0, contract_call_args)?;
        let duration = start.elapsed().as_millis() as u64;
        info!("batch_flow: Batch transfer flow duration: {} ms", duration);
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 4)? {
            return Err(anyhow::format_err!("mint timeout waiting for checkpoint"));
        }

        info!("batch_flow: Start to execute claim contract call");
        for i in 0..user_info.len() {
            let claim_contract_call_args = vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_claim".to_string(),
                inputs: vec![from_user_id],
            }];
            info!(
                "Start to execute claim contract call for user {}",
                user_info[i].pub_key,
            );
            let start = Instant::now();
            for _ in 0..3 {
                if let Err(err) = self.exec_contract_call(user_info[i].pub_key, claim_contract_call_args.clone())
                    .map_err(|err| anyhow::format_err!("exec_contract_call: {}", err)){
                    error!("❌ Task {} - Claim contract call error: {}",i, err);
                    continue;
                }
                break;
            }

            let duration = start.elapsed().as_millis() as u64;
            info!("🔄 Task {} - Claim contract call duration: {} ms",i, duration);
            info!("End to execute claim contract call for user {}", user_info[i].pub_key);
        }

        info!("batch_flow: claim contract call finished, start to wait for checkpoint");
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 1)? {
            return Err(anyhow::format_err!("claim timeout waiting for checkpoint"));
        }

        info!("batch_flow: end call");
        Ok(())
    }

    pub fn multi_user_mint(&self, count: u64) -> Result<()> {
        let user_info = self.register_batch_user(count)?;
        let mint_amount = 250000000000u64;
        for i in 0..user_info.len() {
            {self.wallet_session.write().add_user(user_info[i].pk.clone())?;}
            let public_key = user_info[i].pub_key.clone();
            match self.exec_contract_call(public_key, vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_mint".to_string(),
                inputs: vec![mint_amount],
            }]){
                Ok(_) => {
                    info!("✅ Task {} - Mint contract call success",i);
                },
                Err(err) => {
                    error!("❌ Task {} - Mint contract call error: {}",i, err);
                }
            }
        }
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("claim timeout waiting for checkpoint"));
        }
        info!("multi_user_mint: end call");
        Ok(())
    }

    pub fn deploy_contract(&self, count: u64, mut contract_path: String) -> Result<()> {
        let mint_amount = 250000000000u64;
        let (from_user_id, public_key0) = self.init_user0(mint_amount)?;
        if contract_path.is_empty() {
            contract_path = "../examples/target/examples.json".to_string();
        }
        let defs_array: Vec<DPNFunctionCircuitDefinition> =
            serde_json::from_str(&fs::read_to_string(contract_path)?)?;
        tracing::info!("deploying contract");
        self.wallet_session.read().deploy_contract(public_key0, defs_array)?;
        tracing::info!("contract deployed");
        Ok(())
    }

    fn init_user0(&self, mint_amount: u64) -> Result<(u64, QHashOut<GoldilocksField>)> {
        let pk0 = QHashOut::from_string_or_panic(USER0_PRIVATE_KEY);
        let public_key0 = QHashOut::from_string_or_panic(USER0_SECP_ZK_PUBLIC_KEY);
        let from_user_id = {self.wallet_session.read().st_provider.get_user_id(public_key0)?};
        {self.wallet_session.write().add_user(pk0)?;}
        info!("Start to execute mint contract call");
        self.exec_contract_call(public_key0, vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_mint".to_string(),
            inputs: vec![mint_amount],
        }]);
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        Ok((from_user_id, public_key0))
    }

    pub fn multi_user_transfer(&self, count: u64) -> Result<()> {
        let user_info = self.register_batch_user(count)?;
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        let mut contract_call_args = vec![];
        let transfer_amount = 250000000000u64; //1000 000000000
        let (from_user_id, public_key0) = self.init_user0(transfer_amount * 2 * count)?;
        for i in 0..user_info.len() {
            contract_call_args.push(ContractCallArgs {
                contract_id: 0,
                method_name: "simple_transfer".to_string(),
                inputs: vec![user_info[i].user_id, transfer_amount],
            });
            {self.wallet_session.write().add_user(user_info[i].pk.clone())?;}
        }
        info!("Start to execute transfer contract call");
        self.exec_contract_call(public_key0, contract_call_args)?;
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 4)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        for i in 0..user_info.len() {
            info!("Start to execute claim contract call for user id {}", user_info[i].user_id);
            self.exec_contract_call(user_info[i].pub_key, vec![ContractCallArgs {
                contract_id: 0,
                method_name: "simple_claim".to_string(),
                inputs: vec![from_user_id],
            }])?;
        }
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        info!("multi_user_transfer: end call");
        Ok(())
    }

    pub fn transfer(&self, to_user_id: u64, to_user_public_key: QHashOut<GoldilocksField>, amount: u64) -> Result<()> {
        let (from_user_id, public_key0) = self.init_user0(amount)?;
        self.exec_contract_call(public_key0, vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_transfer".to_string(),
            inputs: vec![to_user_id, amount],
        }])?;
        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        self.exec_contract_call(to_user_public_key, vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_claim".to_string(),
            inputs: vec![from_user_id],
        }])?;

        if !wait_for_new_block(&self.wallet_session.read().st_provider, 2)? {
            return Err(anyhow::format_err!("transfer timeout waiting for checkpoint"));
        }
        info!("transfer: end call");
        Ok(())
    }

}
