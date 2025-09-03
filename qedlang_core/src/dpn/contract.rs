use crate::dpn::vm::def::DPNFunctionCircuitDefinition;
use qed_core::config::network_constants::VM_TYPE_STANRDARD_DAPEN_V1;
use qed_data::qdata::contract::ContractFunctionCodeDefinition;

pub fn dapen_fc_to_cfc_code_definition(
    dpn_fc_def: &DPNFunctionCircuitDefinition,
) -> ContractFunctionCodeDefinition {
    ContractFunctionCodeDefinition {
        method_id: dpn_fc_def.method_id,
        num_inputs: dpn_fc_def.circuit_inputs.len() as u32,
        num_outputs: dpn_fc_def.circuit_outputs.len() as u32,
        vm_type: VM_TYPE_STANRDARD_DAPEN_V1,
        code: serde_cbor::to_vec(dpn_fc_def).unwrap(),
    }
}

pub fn cfc_code_definition_to_dapen_fc(
    cfc_def: &ContractFunctionCodeDefinition,
) -> anyhow::Result<DPNFunctionCircuitDefinition> {
    let res = serde_cbor::from_slice::<DPNFunctionCircuitDefinition>(&cfc_def.code);

    match res {
        Ok(r) => Ok(r),
        Err(e) => anyhow::bail!("error deserializing dapen function definition {:?}", e),
    }
}

