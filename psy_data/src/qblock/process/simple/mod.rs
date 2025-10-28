#![cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

use kvq::traits::KVQBinaryStore;
use plonky2::field::types::{Field, PrimeField64};
use qed_core::config::network_constants::DEFAULT_USER_STATE_TREE_ROOT;
use qed_core::data::qhashout::QHashOut;
use psy_crypto::hash::traits::
    qhashable::QFieldHashable
;
use crate::qblock::process::witnesses::QEDCheckpointStateTransitionCircuitInput;
use crate::{
    protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints,
    qblock::{
        cmds::{core::QEDBlockCommands, register_user::QBCRegisterUser, deploy_contract::QBCDeployContract},
        process::witnesses::{
            QEDDeployContractCircuitInput, QEDInternalBlockCircuitInputs,
            QEDUserRegistrationCircuitInput,
        },
    },
    qdata::{checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDCheckpointLeafStats}, contract::{QEDContractLeaf, ContractCodeDefinition, ContractFunctionCodeDefinition}, user::QEDUserLeaf},
};

use crate::{
    config::store_config::{QEDFelt, QEDHasher},
    models::user::contract_state_tree::UserContractStateTreeId,
    traits::qdatastore::{
        qmetadata::{QMetaDataStoreReaderSync, QMetaDataStoreWriterSync},
        qtreedata::{QTreeDataStoreReaderSync, QTreeDataStoreWriterSync},
    },
};


pub struct SimpleBlockProcessor {}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl SimpleBlockProcessor {
    pub async fn process_block<S: KVQBinaryStore>(
        store: &S,
        cmds: &QEDBlockCommands<QEDFelt>,
        fingerprints: &QEDWorkerToolboxCoreCircuitFingerprints<QEDFelt>,
    ) -> anyhow::Result<QEDInternalBlockCircuitInputs<QEDFelt>> {
        let current_block_state = store.get_latest_l2_block_state().await?;

        let old_checkpoint_id = current_block_state.checkpoint_id;
        let old_checkpoint_leaf = store.get_checkpoint_leaf_data(old_checkpoint_id).await?;
        let old_state_roots = QEDCheckpointGlobalStateRoots{
            contract_tree_root: store.get_contract_tree_root(old_checkpoint_id).await?,
            deposit_tree_root: store.get_deposit_tree_root(old_checkpoint_id).await?,
            user_tree_root: store.get_user_tree_root(old_checkpoint_id).await?,
            withdrawal_tree_root: store.get_withdrawal_tree_root(old_checkpoint_id).await?,
            user_registration_tree_root: store.get_user_registration_tree_root(old_checkpoint_id).await?,
        };
        let new_checkpoint_id = old_checkpoint_id+1;
        let new_checkpoint_id_f = QEDFelt::from_canonical_u64(new_checkpoint_id);

        let mut new_block_state = current_block_state.clone();
        let mut witness_register_users: Vec<QEDUserRegistrationCircuitInput<QEDFelt>> =
            Vec::with_capacity(cmds.register_users.len());
        let mut witness_deploy_contracts: Vec<QEDDeployContractCircuitInput<QEDFelt>> =
            Vec::with_capacity(cmds.deploy_contracts.len());



        for (i, r) in cmds.register_users.iter().enumerate() {
            let user_id = current_block_state.next_user_id + i as u64;
            let user = QEDUserLeaf {
                public_key: r.get_public_key::<QEDHasher>(),
                user_state_tree_root: DEFAULT_USER_STATE_TREE_ROOT,
                balance: QEDFelt::ZERO,
                nonce: QEDFelt::ZERO,
                last_checkpoint_id: new_checkpoint_id_f,
                event_index: QEDFelt::ZERO,
                user_id: QEDFelt::from_canonical_u64(user_id),
            };

            let leaf_hash = user.qfhash::<QEDHasher>();

            store.set_user_leaf_data(new_checkpoint_id,&user)?;
            let user_reg_delta_merkle_proof =
                store.set_user_tree_leaf_hash(new_checkpoint_id, user_id, leaf_hash)?;
            let user_reg_witness = QEDUserRegistrationCircuitInput {
                allowed_circuit_hashes_root: fingerprints
                    .op_register_user
                    .allowed_circuit_hashes_root,
                user_tree_delta_merkle_proof: user_reg_delta_merkle_proof,
                user_leaf: user,
            };
            witness_register_users.push(user_reg_witness);
        }
        let boundry_user_id = new_block_state.next_user_id;
        let boundry_user_registration_merkle_proof = store.get_user_tree_merkle_proof(new_checkpoint_id, boundry_user_id).await?;
        new_block_state.next_user_id += cmds.register_users.len() as u64;

        for (i, d) in cmds.deploy_contracts.iter().enumerate() {
            let contract_id = (current_block_state.next_contract_id as u64) + i as u64;
            let function_tree_root = store.set_contract_function_whitelist(
                new_checkpoint_id,
                contract_id,
                &d.function_whitelist,
            )?;

            let contract_leaf = QEDContractLeaf {
                deployer: d.deployer,
                function_tree_root,
                state_tree_height: QEDFelt::from_canonical_u16(d.code_definition.state_tree_height),
            };
            let contract_leaf_hash = contract_leaf.qfhash::<QEDHasher>();
            store.set_contract_leaf_data(new_checkpoint_id, contract_id, &contract_leaf)?;

            store.set_contract_code_definition(
                new_checkpoint_id,
                contract_id,
                &d.code_definition,
            )?;
            let contract_reg_delta_merkle_proof = store.set_contract_tree_leaf_hash(
                new_checkpoint_id,
                contract_id,
                contract_leaf_hash,
            )?;
            let contract_reg_witness = QEDDeployContractCircuitInput {
                allowed_circuit_hashes_root: fingerprints
                    .op_deploy_contract
                    .allowed_circuit_hashes_root,
                contract_tree_delta_merkle_proof: contract_reg_delta_merkle_proof,
                contract_leaf,
            };
            witness_deploy_contracts.push(contract_reg_witness);
        }
        new_block_state.next_contract_id += cmds.deploy_contracts.len() as u32;

        for upd_user in cmds.update_users.iter() {
            let user_id = upd_user.updated_leaf.user_id.to_canonical_u64();

            let user = store.get_user_leaf_data(new_checkpoint_id, user_id).await?;
            if !user.public_key.eq(&upd_user.updated_leaf.public_key)
                || user.last_checkpoint_id.to_canonical_u64()
                    >= upd_user.updated_leaf.last_checkpoint_id.to_canonical_u64()
                || user.event_index.to_canonical_u64()
                    > upd_user.updated_leaf.event_index.to_canonical_u64()
                || user.nonce.to_canonical_u64() >= upd_user.updated_leaf.nonce.to_canonical_u64()
            {
                anyhow::bail!("cannot change user public key in update");
            }

            let mut last_uct_root = QHashOut::ZERO;

            // start user data updates, this will be on da nodes in production
            for cs_upd in upd_user.contract_state_updates.iter() {
                if cs_upd.updates.len() == 0 {
                    continue;
                }
                let contract_id = cs_upd.contract_id as u64;
                let contract_state_height = store
                    .get_contract_leaf_data(contract_id).await?
                    .state_tree_height
                    .to_canonical_u64() as u8;
                let ucst = UserContractStateTreeId::<S>::new(
                    user_id,
                    contract_id as u32,
                    contract_state_height,
                );
                let mut last_root = QHashOut::ZERO;
                for upd in cs_upd.updates.iter() {
                    last_root = ucst
                        .set_leaf_ucs(store, new_checkpoint_id, upd.state_slot_id, upd.value)?
                        .new_root;
                }
                last_uct_root = store
                    .set_user_contract_tree_leaf_hash(
                        new_checkpoint_id,
                        user_id,
                        contract_id as u32,
                        last_root,
                    )?
                    .new_root;
            }
            if !last_uct_root.eq(&QHashOut::ZERO) {
                if !upd_user
                    .updated_leaf
                    .user_state_tree_root
                    .eq(&last_uct_root)
                {
                    anyhow::bail!("User state tree root mismatch");
                }
            }
            store.set_user_leaf_data(new_checkpoint_id, &upd_user.updated_leaf)?;
            let user_leaf_hash = user.qfhash::<QEDHasher>();
            let user_delta_merkle_proof = store.set_user_tree_leaf_hash(
                new_checkpoint_id,
                upd_user.updated_leaf.user_id.to_canonical_u64(),
                user_leaf_hash,
            )?;

            //r_users.push(user_witness);
        }

        let boundry_user_update_merkle_proof= store.get_user_tree_merkle_proof(new_checkpoint_id, boundry_user_id).await?;
        new_block_state.checkpoint_id = new_checkpoint_id;
        store.set_l2_block_state(&new_block_state)?;

        let new_state_roots = QEDCheckpointGlobalStateRoots{
            contract_tree_root: store.get_contract_tree_root(new_checkpoint_id).await?,
            deposit_tree_root: store.get_deposit_tree_root(new_checkpoint_id).await?,
            user_tree_root: store.get_user_tree_root(new_checkpoint_id).await?,
            withdrawal_tree_root: store.get_withdrawal_tree_root(new_checkpoint_id).await?,
            user_registration_tree_root: store.get_user_registration_tree_root(new_checkpoint_id).await?,
        };
        let mut new_leaf_stats = QEDCheckpointLeafStats::<QEDFelt>::new_empty();
        new_leaf_stats.block_time = QEDFelt::from_canonical_u64(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        new_leaf_stats.user_ops_processed = QEDFelt::from_canonical_u64(cmds.update_users.len() as u64);
        new_leaf_stats.total_transactions = QEDFelt::from_canonical_u64(cmds.update_users.len() as u64 + cmds.register_users.len() as u64 + cmds.deploy_contracts.len() as u64);

        let new_global_state_hash = new_state_roots.qfhash::<QEDHasher>();
        let new_checkpoint_leaf = QEDCheckpointLeaf {
            stats: new_leaf_stats,
            global_chain_root: new_global_state_hash,
        };
        let new_checkpoint_leaf_hash = new_checkpoint_leaf.qfhash::<QEDHasher>();
        store.set_checkpoint_leaf_data(new_checkpoint_id, &new_checkpoint_leaf)?;

        let checkpoint_delta_merkle_proof = store.set_checkpoint_tree_leaf_hash(new_checkpoint_id, new_checkpoint_leaf_hash)?;
        //println!("checkpoint_delta_merkle_proof: {}", serde_json::to_string_pretty(&checkpoint_delta_merkle_proof).unwrap());



        let checkpoint_state_transition = QEDCheckpointStateTransitionCircuitInput{
            old_state_roots,
            old_checkpoint_leaf,
            new_state_roots,
            new_checkpoint_leaf,
            boundry_user_registration_merkle_proof,
            boundry_user_update_merkle_proof,
            checkpoint_delta_merkle_proof,
        };



        Ok(QEDInternalBlockCircuitInputs {
            register_users: witness_register_users,
            deploy_contracts: witness_deploy_contracts,
            checkpoint_state_transition,
        })
    }

    pub async fn prepare_environment_with_real_contract<S: KVQBinaryStore>(
        new_user_public_keys: Vec<QBCRegisterUser<QEDFelt>>,
        deploy_contracts: Vec<QBCDeployContract<QEDFelt>>,
        store: S,
    ) -> anyhow::Result<S> {
        let fake_code_hash = QHashOut::rand();
        let whitelist_items_fake = vec![
            QHashOut::rand(),
            QHashOut::rand(),
            fake_code_hash,
            QHashOut::from_values(0, 0, 0, 0),
        ];
        let fake_code_hash_2 = QHashOut::rand();

        // Initialize store with genesis state if not already initialized
        let dummy_fingerprints = QEDWorkerToolboxCoreCircuitFingerprints::default();
        
        let mut all_users = vec![
            QBCRegisterUser::new_from_u64s([1; 4], [1; 4]),
            QBCRegisterUser::new_from_u64s([1; 4], [13371, 13372, 13373, 13374]),
            QBCRegisterUser::new_from_u64s([1; 4], [13375, 13376, 13377, 13378]),
            QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
            QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
        ];
        all_users.extend(new_user_public_keys);

        let mut all_contracts = vec![
            QBCDeployContract {
                deployer: QBCRegisterUser::new_from_u64s([1; 4], [13371, 13372, 13373, 13374])
                    .get_public_key::<QEDHasher>(),
                code_definition: ContractCodeDefinition {
                    state_tree_height: 12 as u16,
                    functions: vec![ContractFunctionCodeDefinition::default()],
                },
                function_whitelist: whitelist_items_fake.clone(),
            },
            QBCDeployContract {
                deployer: QBCRegisterUser::new_from_u64s([1; 4], [13375, 13376, 13377, 13378])
                    .get_public_key::<QEDHasher>(),
                code_definition: ContractCodeDefinition {
                    state_tree_height: 13 as u16,
                    functions: vec![ContractFunctionCodeDefinition::default()],
                },
                function_whitelist: vec![
                    QHashOut::rand(),
                    QHashOut::rand(),
                    fake_code_hash_2,
                    QHashOut::from_values(0, 0, 0, 0),
                ],
            },
        ];
        all_contracts.extend(deploy_contracts);

        Self::process_block(
            &store,
            &QEDBlockCommands {
                register_users: all_users,
                deploy_contracts: all_contracts,
                update_users: vec![],
            },
            &dummy_fingerprints,
        ).await?;

        // Process additional empty blocks for testing
        Self::process_block(
            &store,
            &QEDBlockCommands {
                register_users: vec![
                    QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                    QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                ],
                deploy_contracts: vec![],
                update_users: vec![],
            },
            &dummy_fingerprints,
        ).await?;

        Self::process_block(
            &store,
            &QEDBlockCommands {
                register_users: vec![
                    QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                    QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                ],
                deploy_contracts: vec![],
                update_users: vec![],
            },
            &dummy_fingerprints,
        ).await?;

        Ok(store)
    }
}
