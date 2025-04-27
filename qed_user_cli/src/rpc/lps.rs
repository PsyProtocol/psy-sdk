use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use qed_data::qdata::user;
use qed_store::traits::qdatastore::{
    qmetadata::QMetaDataStoreReaderSync,
    qtreedata::{QEDComboDataStoreReaderSync, QTreeDataStoreReaderSync},
};

use crate::qed_rpc_call_back;

use super::request::*;
use anyhow::Ok;

use super::provider::RpcProvider;

type F = GoldilocksField;

impl QTreeDataStoreReaderSync<F> for RpcProvider {
    fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractStateTreeRootRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_state_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractStateTreeRootFRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
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
            RequestParams::<F>::GetUserContractStateTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractStateTreeLeafHashFRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
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
            RequestParams::<F>::GetUserContractStateTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractStateTreeMerkleProofFRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
            height,
            leaf_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractStateTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeRootRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractTreeRootFRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeLeafHashRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractTreeLeafHashFRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(get_hash) => Ok(get_hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserContractTreeMerkleProofRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserContractTreeMerkleProofFRPCRequest {
            checkpoint_id,
            user_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserContractTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        // TBD
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QUserRegistrationTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QUserRegistrationTreeRootFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QUserRegistrationTreeLeafHashRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QUserRegistrationTreeLeafHashFRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QUserRegistrationTreeMerkleProofRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_registration_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QUserRegistrationTreeMerkleProofFRPCRequest {
            checkpoint_id,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserRegistrationTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QUserTreeRootRPCRequest { checkpoint_id };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetUserTreeRoot(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QUserTreeRootFRPCRequest { checkpoint_id };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetUserTreeRootF(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserTreeLeafHashRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserTreeLeafHashFRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserTreeMerkleProofRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserTreeMerkleProofFRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QUserSubTreeMerkleProofRPCRequest {
            checkpoint_id,
            root_level,
            leaf_level,
            leaf_index,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetUserSubTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeRootRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_root_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeRootFRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeLeafHashRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeLeafHashFRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeMerkleProofRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractFunctionTreeMerkleProofFRPCRequest {
            checkpoint_id,
            contract_id,
            function_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractFunctionTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeRootFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeLeafHashRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeLeafHashFRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeMerkleProofRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractTreeMerkleProofFRPCRequest {
            checkpoint_id,
            contract_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeRootRPCRequest { checkpoint_id };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetDepositTreeRoot(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeRootFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeLeafHashRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeLeafHashFRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeMerkleProofRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_deposit_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QDepositTreeMerkleProofFRPCRequest {
            checkpoint_id,
            deposit_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetDepositTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeRootFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeLeafHashRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeLeafHashFRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeMerkleProofRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_withdrawal_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QWithdrawalTreeMerkleProofFRPCRequest {
            checkpoint_id,
            withdrawal_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetWithdrawalTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_latest_checkpoint_tree_root(
        &self,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QLatestCheckpointTreeRootRPCRequest {};
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetLatestCheckpointTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeRootRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeRoot(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeRootFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeRootF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeLeafHashRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeLeafHash(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<qed_core::data::qhashout::QHashOut<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeLeafHashFRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeLeafHashF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetHash(hash) => Ok(hash),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeMerkleProofRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeMerkleProof(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<
        qed_crypto::hash::merkle::core::MerkleProofCore<qed_core::data::qhashout::QHashOut<F>>,
    > {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointTreeMerkleProofFRPCRequest {
            checkpoint_id,
            leaf_checkpoint_id,
        };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointTreeMerkleProofF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetMerkleProof(merkel_proof) => Ok(merkel_proof),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
}

impl QMetaDataStoreReaderSync<F> for RpcProvider {
    fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<user::QEDUserLeaf<F>> {
        let rpc_url = self.get_realm_url(user_id)?;
        let input = QUserLeafDataRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetUserLeafData(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetUserLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_user_leaf_data_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<user::QEDUserLeaf<F>> {
        let rpc_url = self.get_realm_url(user_id.to_canonical_u64())?;
        let input = QUserLeafDataFRPCRequest {
            checkpoint_id,
            user_id,
        };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetUserLeafFData(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetUserLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_leaf_data(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractLeafDataRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractLeafData(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_leaf_data_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<qed_data::qdata::contract::QEDContractLeaf<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractLeafDataFRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractLeafDataF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointLeafDataRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointLeafData(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetCheckpointLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDCheckpointLeaf<F>> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QCheckpointLeafDataFRPCRequest { checkpoint_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetCheckpointLeafDataF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetCheckpointLeaf(leaf) => Ok(leaf),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractCodeDefinitionRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractCodeDefinition(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractCode(contract_code) => Ok(contract_code),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_contract_code_definition_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<qed_data::qdata::contract::ContractCodeDefinition> {
        let rpc_url = &self.config.cooridinator_configs;
        let input = QContractCodeDefinitionFRPCRequest { contract_id };
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetContractCodeDefinitionF(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetContractCode(contract_code) => Ok(contract_code),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_latest_l2_block_state(
        &self,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QLatestL2BlockStateRPCRequest {};
        let response = qed_rpc_call_back!(
            self,
            rpc_url,
            RequestParams::<F>::GetLatestL2BlockState(input)
        );
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetL2BlockState(block_state) => Ok(block_state),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_l2_block_state(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QL2BlockStateRPCRequest { checkpoint_id };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetL2BlockState(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetL2BlockState(block_state) => Ok(block_state),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }

    fn get_l2_block_state_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<qed_data::qdata::checkpoint::QEDL2BlockState> {
        let rpc_url = self.get_realm_url(self.current_user_id)?;
        let input = QL2BlockStateFRPCRequest { checkpoint_id };
        let response =
            qed_rpc_call_back!(self, rpc_url, RequestParams::<F>::GetL2BlockStateF(input));
        match response.result {
            ResponseResult::Success(res) => match res {
                LPSResponse::GetL2BlockState(block_state) => Ok(block_state),
                _ => Err(anyhow::format_err!("rpc call return wrong data")),
            },
            ResponseResult::Error(e) => Err(anyhow::format_err!("rpc call failed `{:?}`", e)),
        }
    }
}

impl QEDComboDataStoreReaderSync<F> for RpcProvider {}
