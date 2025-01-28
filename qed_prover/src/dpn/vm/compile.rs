use anyhow::Ok;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::{HashOut, RichField},
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::{merkle::core::{DeltaMerkleProofCore, MerkleProofCore}, traits::qhashable::QFieldHashable};
use qed_data::dpn::proving_session::DPNProvingSessionSimpleMethodCall;
use qed_store::{
    config::store_config::QEDHasher, controllers::local::proving_session::QEDLocalProvingSessionStore, store::imm::{cmd::{QSRMerkleCmd, QSRMerkleCmdGetUserContractStateTreeMerkleProof, QSRMerkleCmdGetUserContractTreeMerkleProof}, cmd_processor::{DPNInvokeDeferredMethodCallWitness, DPNReadOtherUserContractStateLeafMerkleProof, DPNStateCmdWitness, QEDReadCommandProcessorSync, QEDReadCommandProcessorSyncMut}}
};
use qedlang_core::dpn::{
    ops::{
        op_types::DPNOpType,
        state_cmd::{data::DPNStateCmd, types::DPNStateCmdCore},
    },
    vm::{def::DPNFunctionCircuitDefinition, exec::SimpleDPNExecutor},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QEDCmdWithInputAndWitness {
    pub state_cmd: DPNStateCmd<u64>,
    pub witness: DPNStateCmdWitness<F>,
    pub result: Vec<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(bound = "for<'de2> F: Deserialize<'de2>")]
pub struct QEDEvalSessionResult<F: RichField> {
    //output: Vec<F>,
    pub cmd_witnesses: Vec<QEDCmdWithInputAndWitness<F>>,
}

impl<F: RichField> QEDEvalSessionResult<F> {
    pub fn new() -> Self {
        Self {
            cmd_witnesses: Vec::new(),
        }
    }
}
type GF = GoldilocksField;
impl QEDEvalSessionResult<GF> {
    pub fn process_state_cmd<R: QEDReadCommandProcessorSync<GF>>(
        &mut self,
        executor: &mut SimpleDPNExecutor<GF>,
        sesh: &mut QEDLocalProvingSessionStore<GF, R>,
        cmd: &DPNStateCmd<u64>,
    ) -> anyhow::Result<()> {
        let real_inputs = cmd
            .get_inputs()
            .iter()
            .map(|x| executor.resolve_target(*x).to_canonical_u64())
            .collect::<Vec<u64>>();
        let new_cmd = cmd.convert_to_u64(&real_inputs);

        let r = sesh.resolve_vec(&new_cmd)?;
        self.cmd_witnesses.push(r);
        Ok(())
    }

    pub fn eval_session<R: QEDReadCommandProcessorSync<GF>>(
        &mut self,
        fn_def: &DPNFunctionCircuitDefinition,
        sesh: &mut QEDLocalProvingSessionStore<GF, R>,
        inputs: Vec<Target>,
    ) -> anyhow::Result<Vec<GF>> {
        let mut executor = SimpleDPNExecutor::<GF>::new_with_contract_ctx(
            inputs,
            sesh.user_id,
            sesh.current_contract_id,
            sesh.start_checkpoint,
            sesh.nonce,
        );
        let state_cmd_len = fn_def.state_command_resolution_indices.len();
        let mut next_state_cmd_id = 0;
        let mut next_state_cmd_index = if state_cmd_len == 0 {
            fn_def.definitions.len() + 10
        } else {
            fn_def.state_command_resolution_indices[0]
        };
        for (i, def) in fn_def.definitions.iter().enumerate() {
            if def.op_type.eq(&DPNOpType::GetStateCommandResultSingle) {
                let ind = (def.inputs[0] as usize);
                executor.push_external_target(self.cmd_witnesses[ind].result[0]);
            } else if def.op_type.eq(&DPNOpType::GetStateCommandResultArray) {
                let ind = (def.inputs[0] as usize);
                executor.push_external_target_array(self.cmd_witnesses[ind].result.clone());
            } else if def.op_type.eq(&DPNOpType::GetStateCommandResultHash) {
                let ind = (def.inputs[0] as usize);
                executor.push_external_hash([
                    self.cmd_witnesses[ind].result[0],
                    self.cmd_witnesses[ind].result[1],
                    self.cmd_witnesses[ind].result[2],
                    self.cmd_witnesses[ind].result[3],
                ]);
            } else {
                executor.process_var_def(&def);
            }
            while (i + 1) >= next_state_cmd_index {
                self.process_state_cmd(
                    &mut executor,
                    sesh,
                    &fn_def.state_commands[next_state_cmd_id],
                )?;
                next_state_cmd_id += 1;
                if next_state_cmd_id >= state_cmd_len {
                    next_state_cmd_index = fn_def.definitions.len() + 10;
                } else {
                    next_state_cmd_index =
                        fn_def.state_command_resolution_indices[next_state_cmd_id];
                }
            }
        }
        for assertion in fn_def.assertions.iter() {
            let left = executor.resolve_target(assertion.left).to_canonical_u64();
            let right = executor.resolve_target(assertion.right).to_canonical_u64();
            if left != right {
                anyhow::bail!(
                    "assertion failed: {} (left: {}, right: {})",
                    assertion.message,
                    left,
                    right
                );
            }
        }
        let outputs = fn_def
            .circuit_outputs
            .iter()
            .map(|x| executor.resolve_target(*x))
            .collect::<Vec<GF>>();

        Ok(outputs)
    }
}
