use std::str::FromStr;

use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::poseidon::PoseidonHash,
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
    ups::circuits::LocalCircuitType,
};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_prover::ups::{
    circuit_manager::core::QEDUPSStepCircuitManager, session::UserProvingSessionManager,
};
use qed_store::{
    config::store_config::QEDHasher,
    controllers::local::{
        proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore,
    },
    store::imm::cmd_processor::QEDReadCommandProcessorSync,
    traits::qdatastore::qmetadata::QMetaDataStoreReaderSync,
};

use crate::rpc::{
    provider::{QUserRpcProvider, RpcConfig, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};

use super::{
    args::{ContractCallArgs, WalletSessionArgs},
    utils::prove_func,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub struct WalletSession {
    pub wallet: SimpleQEDZKSignatureManager<C, D>,
    pub mgr: UserProvingSessionManager<F, PoseidonHash, RpcProvider, C, D>,
    pub main_circuits: QEDUPSStepCircuitManager<C, D>,
    pub circuit_info: SessionCircuitInfoStore<F>,
    pub st_provider: RpcProvider,
    pub private_key: QHashOut<GoldilocksField>,
    pub user_id: u64,

    pub nonce: F,
    pub current_checkpoint_id: u64,
}

impl WalletSession {
    pub fn new(
        rpc_config: &RpcConfig,
        private_key: QHashOut<GoldilocksField>,
    ) -> anyhow::Result<Self> {
        tracing::info!("init rpc provider");
        let mut st_provider = RpcProvider::new_with_config(rpc_config)?;

        tracing::info!("init wallet");
        let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

        let zkey_info = wallet.add_private_key_get_info(SimpleQEDPrivateKey { private_key });

        let user_id = st_provider.get_user_id(zkey_info.public_key_param)?;
        st_provider.current_user_id = user_id;
        tracing::info!("user id: {}", user_id);

        let latest_l2_block_state = st_provider.resolve_get_latest_l2_block_state()?;
        tracing::info!(
            "latest l2 block state: {}",
            serde_json::to_string_pretty(&latest_l2_block_state)?
        );

        tracing::info!("init ups step circuit manager");
        let main_circuits =
            QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST);
        let mut circuit_info = SessionCircuitInfoStore::new();

        tracing::info!("register ZKSignature circuit info");
        circuit_info.register_circuit(
            LocalCircuitType::SimpleZKSignature.into(),
            wallet.circuit.get_fingerprint(),
            wallet.circuit.get_verifier_config_ref().into(),
        );

        main_circuits.register_info(&mut circuit_info);

        tracing::info!("get new nonce");
        let new_nonce = st_provider
            .get_user_leaf_data(latest_l2_block_state.checkpoint_id, user_id)
            .map_err(|e| anyhow::format_err!("{}", e.to_string()))?
            .nonce
            + GoldilocksField::from_noncanonical_u64(1);

        tracing::info!("create local proving session store");
        let lps = QEDLocalProvingSessionStore::new_at(
            st_provider.clone(),
            GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
            F::from_canonical_u64(user_id),
            new_nonce,
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
        );

        tracing::info!("create ups manager");
        let mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
            lps,
            circuit_info.clone(),
            main_circuits.ups_circuit_whitelist_root,
        )?;

        Ok(WalletSession {
            wallet,
            main_circuits,
            circuit_info,
            st_provider,
            private_key,
            user_id,
            mgr,
            nonce: new_nonce,
            current_checkpoint_id: latest_l2_block_state.checkpoint_id,
        })
    }

    pub fn start_session(&mut self) -> anyhow::Result<()> {
        tracing::info!("start new user proving session");
        let latest_l2_block_state = self.st_provider.resolve_get_latest_l2_block_state()?;
        let new_nonce = self
            .st_provider
            .get_user_leaf_data(latest_l2_block_state.checkpoint_id, self.user_id)
            .map_err(|e| anyhow::format_err!("{}", e.to_string()))?
            .nonce
            + GoldilocksField::from_noncanonical_u64(1);

        if latest_l2_block_state.checkpoint_id != self.current_checkpoint_id
            || new_nonce != self.nonce
        {
            tracing::info!(
                "checkpoint {} -> {}, nonce {} -> {}. reset session.",
                self.current_checkpoint_id,
                latest_l2_block_state.checkpoint_id,
                self.nonce.to_noncanonical_u64(),
                new_nonce.to_noncanonical_u64()
            );
            self.current_checkpoint_id = latest_l2_block_state.checkpoint_id;

            self.nonce = new_nonce;

            let lps = QEDLocalProvingSessionStore::new_at(
                self.st_provider.clone(),
                GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
                F::from_canonical_u64(self.user_id),
                self.nonce,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
            );

            tracing::info!("create user proving session manager");
            self.mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
                lps,
                self.circuit_info.clone(),
                self.main_circuits.ups_circuit_whitelist_root,
            )?;
        }

        tracing::info!("local proving ups start");

        self.mgr.prove_ups_start(&self.main_circuits)?;

        Ok(())
    }

    pub fn prove_contract_call(
        &mut self,
        contract_call_arg: ContractCallArgs,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "prove contract call at contract {}, method {}",
            contract_call_arg.contract_id,
            contract_call_arg.method_name
        );
        prove_func(
            &self.st_provider,
            &self.main_circuits,
            &mut self.mgr,
            contract_call_arg.contract_id,
            &contract_call_arg.method_name,
            contract_call_arg
                .inputs
                .iter()
                .map(|x| GoldilocksField::from_noncanonical_u64(*x))
                .collect(),
        )
    }

    pub fn prove_contract_calls(
        &mut self,
        contract_call_args: Vec<ContractCallArgs>,
    ) -> anyhow::Result<()> {
        for contract_call_arg in contract_call_args {
            self.prove_contract_call(contract_call_arg)?;
        }
        Ok(())
    }

    pub fn sign_and_submit(&mut self) -> anyhow::Result<()> {
        let sighash = self.mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, self.nonce);
        tracing::info!("zk sign for signhash: {}", sighash.to_string());
        let signature_proof = self
            .wallet
            .zk_sign_for_private_key_value(self.private_key, sighash)?;

        self.mgr
            .proof_tree_state
            .finalize_tree(&self.main_circuits.proof_tree_agg_circuits)?;

        let public_key_param =
            SimpleQEDPrivateKey::new(self.private_key).get_public_key_param::<QEDHasher>();
        tracing::info!(
        "prove end cap with network magic {:x}, nonce {}, fingerprint {}, public key param {}, signature proof {:?}",
            QED_NETWORK_MAGIC_REGTEST,
            self.nonce,
            self.wallet.circuit.get_fingerprint(),
            public_key_param,
            signature_proof.public_inputs
        );
        let end_cap_proof = self.mgr.prove_end_cap(
            &self.main_circuits,
            QED_NETWORK_MAGIC_REGTEST,
            self.nonce,
            self.wallet.circuit.get_fingerprint(),
            public_key_param,
            signature_proof,
            self.wallet.circuit.get_verifier_config_ref().to_owned(),
        )?;

        let user_ec_input: SubmitUserEndCapNonProofInput<F> = self.mgr.get_api_input()?;
        tracing::info!(
            "get user ec input: {}",
            serde_json::to_string_pretty(&user_ec_input)?
        );

        // tracing::info!(
        //     "submit end cap proof: {}",
        //     serde_json::to_string_pretty(&end_cap_proof)?
        // );
        let req = QSubmitEndCapRPCRequest {
            user_ec_input,
            proof: end_cap_proof,
        };

        self.st_provider.submit_end_cap_proof::<F>(req)?;

        Ok(())
    }
}

pub fn run(args: WalletSessionArgs) -> anyhow::Result<()> {
    let rpc_config: RpcConfig = serde_json::from_str(&std::fs::read_to_string(args.rpc_config)?)?;
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let contract_call_args: Vec<ContractCallArgs> =
        serde_json::from_str(&std::fs::read_to_string(args.contract_calls)?)?;

    let mut wallet_session = WalletSession::new(&rpc_config, private_key)?;
    wallet_session.start_session()?;
    wallet_session.prove_contract_calls(contract_call_args)?;
    wallet_session.sign_and_submit()?;

    Ok(())
}
