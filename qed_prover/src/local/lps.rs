use super::provider::RpcProvider;
use super::request::*;
use crate::qed_rpc_call_back;
use std::result::Result::Ok;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::{
    config::network_constants::{COORDINATOR_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::qdata::{
    checkpoint::{QEDCheckpointLeaf, QEDL2BlockState},
    contract::{ContractCodeDefinition, QEDContractLeaf, SimpleContractCodeDefinition},
    user::{self, QEDUserLeaf},
};
use qed_data::{
    config::store_config::QEDHasher,
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync,
        qtreedata::{QEDComboDataStoreReaderSync, QTreeDataStoreReaderSync},
    },
};
use tracing::{debug, error, info, instrument};

type F = GoldilocksField;

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl QTreeDataStoreReaderSync<F> for RpcProvider {
    #[instrument(skip(self), fields(checkpoint_id, user_id, contract_id))]
    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            contract_id = contract_id,
            "Fetching user contract state tree root"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractStateTreeRootRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    contract_id = contract_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_state_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(
        skip(self),
        fields(checkpoint_id, user_id, contract_id, height, leaf_id)
    )]
    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            contract_id = contract_id,
            height = height,
            leaf_id = leaf_id,
            "Fetching user contract state tree leaf hash"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractStateTreeLeafHashRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    contract_id = contract_id,
                    height = height,
                    leaf_id = leaf_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_state_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(
        skip(self),
        fields(checkpoint_id, user_id, contract_id, height, leaf_id)
    )]
    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            contract_id = contract_id,
            height = height,
            leaf_id = leaf_id,
            "Fetching user contract state tree merkle proof"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractStateTreeMerkleProofRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    contract_id = contract_id,
                    height = height,
                    leaf_id = leaf_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_state_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, user_id))]
    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            "Fetching user contract tree root"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeRootRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, user_id, contract_id))]
    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            contract_id = contract_id,
            "Fetching user contract tree leaf hash"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeLeafHashRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    contract_id = contract_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_tree_root_f rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, user_id, contract_id))]
    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        debug!(
            checkpoint_id = checkpoint_id,
            user_id = user_id,
            contract_id = contract_id,
            "Fetching user contract tree merkle proof"
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeMerkleProofRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    contract_id = contract_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                debug!("Merkle proof verification result: {}", merkle_proof.verify::<QEDHasher>());
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_contract_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_user_registration_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            "Fetching user registration tree root"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QUserRegistrationTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_registration_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, leaf_index))]
    async fn get_user_registration_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            leaf_index = leaf_index,
            "Fetching user registration tree leaf hash"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QUserRegistrationTreeLeafHashRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    leaf_index = leaf_index,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_registration_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, leaf_index))]
    async fn get_user_registration_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        debug!(
            checkpoint_id = checkpoint_id,
            leaf_index = leaf_index,
            "Fetching user registration tree merkle proof"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QUserRegistrationTreeMerkleProofRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    leaf_index = leaf_index,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_registration_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_user_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(checkpoint_id = checkpoint_id, "Fetching user tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QUserTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, user_id))]
    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!(
            "Fetching user tree leaf hash checkpoint_id: {}, user_id: {}",
            checkpoint_id, user_id
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserTreeLeafHashRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, user_id))]
    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!(
            "Fetching user tree merkle proof checkpoint_id: {}, user_id: {}",
            checkpoint_id, user_id
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserTreeMerkleProofRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(mut merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                debug!("Retrieved bottom merkle proof: {}", serde_json::to_string_pretty(&merkle_proof).unwrap());
                info!("Merkle proof root: {:?}", merkle_proof.root.to_string());
                info!("Merkle proof value: {:?}", merkle_proof.value.to_string());
                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    verify_result = %merkle_proof.verify::<QEDHasher>(),
                    "Before verify"
                );

                let top_proof = self
                    .get_user_sub_tree_merkle_proof(
                        checkpoint_id,
                        0,
                        COORDINATOR_USER_TREE_HEIGHT,
                        self.get_realm_id(user_id),
                    )
                    .await?;
                debug!("Retrieved top proof: {}", serde_json::to_string_pretty(&top_proof).unwrap());
                let mut new_siblings = vec![];
                new_siblings.extend_from_slice(
                    &merkle_proof.siblings[0..(REALM_USER_TREE_HEIGHT as usize)],
                );
                new_siblings.extend_from_slice(&top_proof.siblings);
                merkle_proof.root = top_proof.root;
                merkle_proof.siblings = new_siblings;
                debug!("Modified merkle proof with top proof: {}", serde_json::to_string_pretty(&merkle_proof).unwrap());

                debug!(
                    checkpoint_id = checkpoint_id,
                    user_id = user_id,
                    verify_result = %merkle_proof.verify::<QEDHasher>(),
                    "After verify"
                );
                match merkle_proof.verify::<QEDHasher>() {
                    true => Ok(merkle_proof),
                    false => Err(anyhow::format_err!("user tree merkle proof verify failed")),
                }
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, root_level, leaf_level, leaf_index))]
    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!(
            "Fetching user sub tree merkle proof checkpoint_id: {}, root_level: {}, leaf_level: {}, leaf_index: {}",
            checkpoint_id, root_level, leaf_level, leaf_index
        );

        if root_level > leaf_level {
            anyhow::bail!("root level {} must be less than leaf level {}", root_level, leaf_level);
        } 
        if root_level > GLOBAL_USER_TREE_HEIGHT || leaf_level > GLOBAL_USER_TREE_HEIGHT {
            anyhow::bail!("root level {} and leaf level {} must be less than global user tree height {}", root_level, leaf_level, GLOBAL_USER_TREE_HEIGHT);
        }

        let real_user_id = leaf_index << (GLOBAL_USER_TREE_HEIGHT - leaf_level);
        let realm_rpc_url = self.get_realm_url(real_user_id)?;

        let coordinator_rpc_url = self.get_coordinator_url()?;

        if root_level >= COORDINATOR_USER_TREE_HEIGHT {
            self.get_user_sub_tree_merkle_proof_inner(realm_rpc_url, checkpoint_id, root_level, leaf_level, leaf_index)
                .await
        } else if leaf_level <= COORDINATOR_USER_TREE_HEIGHT {
            self.get_user_sub_tree_merkle_proof_inner(&coordinator_rpc_url, checkpoint_id, root_level, leaf_level, leaf_index)
                .await
        } else {
            tracing::info!("you need to get both sub tree of coordinator and realm");
            let top_tree_leaf_index = leaf_index >> (leaf_level - COORDINATOR_USER_TREE_HEIGHT);
            let top_tree_proof = self
                .get_user_sub_tree_merkle_proof_inner(&coordinator_rpc_url, checkpoint_id, root_level, COORDINATOR_USER_TREE_HEIGHT, top_tree_leaf_index)
                .await?;
            debug!("top tree proof:{}", serde_json::to_string_pretty(&top_tree_proof)?);
            let bottom_tree_proof = self
                .get_user_sub_tree_merkle_proof_inner(&realm_rpc_url, checkpoint_id, COORDINATOR_USER_TREE_HEIGHT, leaf_level, leaf_index)
                .await?;
            debug!("bottom tree proof:{}", serde_json::to_string_pretty(&bottom_tree_proof)?);

            if top_tree_proof.value != bottom_tree_proof.root {
                tracing::error!("coordinator sub tree proof's value {} should be equal to realm sub tree proof's root {}", top_tree_proof.value, bottom_tree_proof.root);
                anyhow::bail!("coordinator sub tree proof's value {} should be equal to realm sub tree proof's root {}", top_tree_proof.value, bottom_tree_proof.root);
            }

            let mut new_siblings = bottom_tree_proof.siblings;
            new_siblings.extend_from_slice(&top_tree_proof.siblings);

            let combine_tree_proof = MerkleProofCore {
                root: top_tree_proof.root,
                value: bottom_tree_proof.value,
                index: leaf_index,
                siblings: new_siblings,
            };

            debug!("combine_tree_proof: {}", serde_json::to_string_pretty(&combine_tree_proof)?);

            debug!(
                checkpoint_id = checkpoint_id,
                root_level = root_level,
                leaf_level = leaf_level,
                leaf_index = leaf_index,
                verify_result = %combine_tree_proof.verify::<QEDHasher>(),
                "After verify"
            );
            match combine_tree_proof.verify::<QEDHasher>() {
                true => Ok(combine_tree_proof),
                false => Err(anyhow::format_err!("user sub tree merkle proof verify failed")),
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, contract_id))]
    async fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            contract_id = contract_id,
            "Fetching contract function tree root"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractFunctionTreeRootRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    contract_id = contract_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_function_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, contract_id, function_id))]
    async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            contract_id = contract_id,
            function_id = function_id,
            "Fetching contract function tree leaf hash"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractFunctionTreeLeafHashRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    contract_id = contract_id,
                    function_id = function_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_function_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, contract_id, function_id))]
    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        debug!(
            checkpoint_id = checkpoint_id,
            contract_id = contract_id,
            function_id = function_id,
            "Fetching contract function tree merkle proof"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractFunctionTreeMerkleProofRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    contract_id = contract_id,
                    function_id = function_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_function_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_contract_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(checkpoint_id = checkpoint_id, "Fetching contract tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, contract_id))]
    async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!(
            "Fetching contract tree leaf hash checkpoint_id: {}, contract_id: {}",
            checkpoint_id, contract_id
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractTreeLeafHashRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    contract_id = contract_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, contract_id))]
    async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!(
            "Fetching contract tree merkle proof checkpoint_id: {}, contract_id: {}",
            checkpoint_id, contract_id
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractTreeMerkleProofRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    contract_id = contract_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_deposit_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(checkpoint_id = checkpoint_id, "Fetching deposit tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QDepositTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_deposit_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, deposit_id))]
    async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        debug!(
            checkpoint_id = checkpoint_id,
            deposit_id = deposit_id,
            "Fetching deposit tree leaf hash"
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QDepositTreeLeafHashRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    deposit_id = deposit_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_deposit_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, deposit_id))]
    async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!("Fetching deposit tree merkle proof");
        let rpc_url = self.get_coordinator_url()?;
        let input = QDepositTreeMerkleProofRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    deposit_id = deposit_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_deposit_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_withdrawal_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!("Fetching withdrawal tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QWithdrawalTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_withdrawal_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, withdrawal_id))]
    async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!("Fetching withdrawal tree leaf hash");
        let rpc_url = self.get_coordinator_url()?;
        let input = QWithdrawalTreeLeafHashRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    withdrawal_id = withdrawal_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_withdrawal_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, withdrawal_id))]
    async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!("Fetching withdrawal tree merkle proof");
        let rpc_url = self.get_coordinator_url()?;
        let input = QWithdrawalTreeMerkleProofRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    withdrawal_id = withdrawal_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_withdrawal_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_latest_checkpoint_tree_root(
        &self,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!("Fetching latest checkpoint tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QLatestCheckpointTreeRootRPCRequest {};
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetLatestCheckpointTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_latest_checkpoint_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_checkpoint_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!("Fetching checkpoint tree root");
        let rpc_url = self.get_coordinator_url()?;
        let input = QCheckpointTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeRoot(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_checkpoint_tree_root rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, leaf_checkpoint_id))]
    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        info!("Fetching checkpoint tree leaf hash");
        let rpc_url = self.get_coordinator_url()?;
        let input = QCheckpointTreeLeafHashRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeLeafHash(input),
            QHashOut<F>
        );
        match response.result {
            ResponseResult::Success(hash) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    leaf_checkpoint_id = leaf_checkpoint_id,
                    hash = %hash,
                    "Successfully fetched hash"
                );
                Ok(hash)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_checkpoint_tree_leaf_hash rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id, leaf_checkpoint_id))]
    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        info!("Fetching checkpoint tree merkle proof");
        let rpc_url = self.get_coordinator_url()?;
        let input = QCheckpointTreeMerkleProofRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeMerkleProof(input),
            MerkleProofCore<QHashOut<F>>
        );
        match response.result {
            ResponseResult::Success(merkle_proof) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    leaf_checkpoint_id = leaf_checkpoint_id,
                    merkle_proof = %serde_json::to_string_pretty(&merkle_proof).unwrap(),
                    "Successfully fetched merkle proof"
                );
                Ok(merkle_proof)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_checkpoint_tree_merkle_proof rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl QMetaDataStoreReaderSync<F> for RpcProvider {
    #[instrument(skip(self), fields(checkpoint_id, user_id))]
    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<user::QEDUserLeaf<F>> {
        info!(
            "Fetching user leaf data checkpoint_id: {}, user_id: {}",
            checkpoint_id, user_id
        );
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserLeafDataRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserLeafData(input),
            QEDUserLeaf<F>
        );
        use qed_crypto::hash::traits::qhashable::QFieldHashable;
        match response.result {
            ResponseResult::Success(leaf) => {
                info!(
                    "Successfully fetched user leaf data checkpoint_id: {}, user_id: {}, leaf: {}, hash: {}",
                    checkpoint_id,
                    user_id,
                    serde_json::to_string_pretty(&leaf).unwrap(),
                    leaf.qfhash::<QEDHasher>().to_string()
                );
                Ok(leaf)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_user_leaf_data rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(contract_id))]
    async fn get_contract_leaf_data(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        info!("Fetching contract leaf data contract_id: {}", contract_id);
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractLeafDataRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractLeafData(input),
            QEDContractLeaf<F>
        );
        match response.result {
            ResponseResult::Success(leaf) => {
                debug!(
                    contract_id = contract_id,
                    leaf = %serde_json::to_string_pretty(&leaf).unwrap(),
                    "Successfully fetched contract leaf"
                );
                Ok(leaf)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_leaf_data rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        info!(
            "Fetching checkpoint leaf data checkpoint_id: {}",
            checkpoint_id
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QCheckpointLeafDataRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointLeafData(input),
            QEDCheckpointLeaf<F>
        );
        match response.result {
            ResponseResult::Success(leaf) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    leaf = %serde_json::to_string_pretty(&leaf).unwrap(),
                    "Successfully fetched checkpoint leaf"
                );
                Ok(leaf)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_checkpoint_leaf_data rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(contract_id))]
    async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        info!(
            "Fetching contract code definition contract_id: {}",
            contract_id
        );
        let rpc_url = self.get_coordinator_url()?;
        let input = QContractCodeDefinitionRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractCodeDefinition(input),
            ContractCodeDefinition
        );
        match response.result {
            ResponseResult::Success(contract_code) => {
                debug!(
                    "Successfully fetched contract {} code definition: {}",
                    contract_id,
                    serde_json::to_string_pretty(&SimpleContractCodeDefinition::from(
                        &contract_code
                    ))?
                );
                Ok(contract_code)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_contract_code_definition rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        info!("Fetching latest L2 block state");
        let rpc_url = self.get_coordinator_url()?;
        let input = QLatestL2BlockStateRPCRequest {};
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetLatestL2BlockState(input),
            QEDL2BlockState
        );
        match response.result {
            ResponseResult::Success(block_state) => {
                debug!(
                    block_state = %serde_json::to_string_pretty(&block_state).unwrap(),
                    "Successfully fetched L2 block state"
                );
                Ok(block_state)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_latest_l2_block_state rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }

    #[instrument(skip(self), fields(checkpoint_id))]
    async fn get_l2_block_state(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        info!("Fetching L2 block state checkpoint_id: {}", checkpoint_id);
        let rpc_url = self.get_coordinator_url()?;
        let input = QL2BlockStateRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetL2BlockState(input),
            QEDL2BlockState
        );
        match response.result {
            ResponseResult::Success(block_state) => {
                debug!(
                    checkpoint_id = checkpoint_id,
                    block_state = %serde_json::to_string_pretty(&block_state).unwrap(),
                    "Successfully fetched L2 block state"
                );
                Ok(block_state)
            }
            ResponseResult::Error(e) => {
                error!("RPC call failed: {:?}", e);
                Err(anyhow::format_err!(
                    "get_l2_block_state rpc call failed `{:?}`",
                    e
                ))
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), maybe_async::maybe_async)]
#[cfg_attr(target_arch = "wasm32", maybe_async::maybe_async(?Send))]
impl QEDComboDataStoreReaderSync<F> for RpcProvider {}
