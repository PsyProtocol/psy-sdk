use crate::dpn::ops::{op_types::DPNBuiltInDataType, state_cmd::{data::DPNStateCmd, types::DPNStateCmdCore}, sym_felt::SymFeltRef, sym_felt_store::SymFeltStore};

use super::def::{encode_indexed_op_id, DPNAssertEqInfoIndexed, DPNIndexedVarDef};

pub struct QEDCompileResult {
    pub circuit_inputs: Vec<u64>,
    pub circuit_outputs: Vec<u64>,
    pub state_commands: Vec<DPNStateCmd<u64>>,
    pub state_command_resolution_indices: Vec<usize>,
    pub assertions: Vec<DPNAssertEqInfoIndexed>,
    pub definitions: Vec<DPNIndexedVarDef>,

    /*
    
    
    Target = 0,
    Bool = 1,
    U32Target = 2,
    HashOut = 3,
    HashOut160 = 4,
    TargetArray = 5,
    BoolArray = 6,
    U32TargetArray = 7, */
    
    pub total_targets: usize,
    pub total_bools: usize,
    pub total_u32s: usize,
    pub total_hashes: usize,
    pub total_hash160s: usize,
    pub total_target_arrays: usize,
    pub total_bool_arrays: usize,
    pub total_u32_arrays: usize,

    pub indexed_map: hashbrown::HashMap<SymFeltRef, u64>,
}


impl QEDCompileResult {
    pub fn new() -> Self {
        QEDCompileResult {
            circuit_inputs: vec![],
            circuit_outputs: vec![],
            state_commands: vec![],
            state_command_resolution_indices: vec![],
            assertions: vec![],
            definitions: vec![],
            total_targets: 0,
            total_target_arrays: 0,
            total_hashes: 0,
            total_u32_arrays: 0,
            total_bools: 0,
            total_bool_arrays: 0,
            total_u32s: 0,
            total_hash160s: 0,
            indexed_map: hashbrown::HashMap::new(),
        }
    }

    pub fn injest_sfr(&mut self, sym_store: &SymFeltStore, value: SymFeltRef) -> u64 {
        if self.indexed_map.contains_key(&value){
            return *self.indexed_map.get(&value).unwrap();
        }



        let mut children_inds = sym_store.get_direct_children(value).into_iter().map(|c| self.injest_sfr(sym_store, c)).collect::<Vec<_>>();


        let new_base_index = match value.get_op_type().get_data_type() {
            DPNBuiltInDataType::Target => {
                let v = self.total_targets;
                self.total_targets+=1;
                v
            },
            DPNBuiltInDataType::Bool => {
                let v = self.total_bools;
                self.total_bools+=1;
                v
            },
            DPNBuiltInDataType::U32Target => {
                let v = self.total_u32s;
                self.total_u32s+=1;
                v
            },
            DPNBuiltInDataType::HashOut => {
                let v = self.total_hashes;
                self.total_hashes+=1;
                v
            },
            DPNBuiltInDataType::HashOut160 => {
                let v = self.total_hash160s;
                self.total_hash160s+=1;
                v
            },
            DPNBuiltInDataType::TargetArray => {
                let v = self.total_target_arrays;
                self.total_target_arrays+=1;
                v
            },
            DPNBuiltInDataType::BoolArray => {
                let v = self.total_bool_arrays;
                self.total_bool_arrays+=1;
                v
            },
            DPNBuiltInDataType::U32TargetArray => {
                let v = self.total_u32_arrays;
                self.total_u32_arrays+=1;
                v
            },
            DPNBuiltInDataType::Unknown => panic!("unsupported data type")
        };
        if !value.needs_store() {
            let idef = value.get_inline_def();
            self.definitions.push(DPNIndexedVarDef{
                data_type: idef.op_type.get_data_type(),
                index: new_base_index,
                op_type: idef.op_type,
                inputs: vec![idef.const_param]
            });
        }else {
            let vdef = sym_store.get(value);
            let mut inputs = if value.get_op_type().has_constant_param() {
                vec![vdef.const_param]
            }else{
                vec![]
            };
            inputs.append(&mut children_inds);
            self.definitions.push(DPNIndexedVarDef{
                data_type: vdef.op_type.get_data_type(),
                index: new_base_index,
                op_type: vdef.op_type,
                inputs
            });
        }

        let new_op_id = encode_indexed_op_id(value.get_op_type().get_data_type(), new_base_index);
        self.indexed_map.insert(value, new_op_id);
        new_op_id
    }

    pub fn injest_state_cmd(&mut self, sym_store: &SymFeltStore, cmd: DPNStateCmd<SymFeltRef>) {
        let inputs = cmd.get_inputs();
        let inputs_resolved = inputs.iter().map(|c| self.injest_sfr(sym_store, *c)).collect::<Vec<_>>();
        let def_count = self.definitions.len();
        self.state_command_resolution_indices.push(def_count);
        let converted = cmd.convert_to_u64(&inputs_resolved);
        self.state_commands.push(converted);
    }
}



/*

QExecContext*/