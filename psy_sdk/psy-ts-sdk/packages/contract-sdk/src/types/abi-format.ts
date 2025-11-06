// packages/codegen/src/types/abi-format.ts

export interface AbiFormat {
    version: string;
    structs: StructDefinition[];
}

export interface StructDefinition {
    name: string;
    is_contract: boolean;
    fields: FieldDefinition[];
    functions?: FunctionDefinition[];
}

export interface FieldDefinition {
    name: string;
    type: FieldType;
}

export type FieldType = string | ArrayType;

export interface ArrayType {
    type: "Array";
    inner_type: string;
    length: number;
}

export interface FunctionDefinition {
    name: string;
    params: ParamDefinition[];
    return: ReturnDefinition[];
}

export interface ParamDefinition {
    name: string;
    type: string;
}

export interface ReturnDefinition {
    name?: string;
    type: string;
}

// Internal representation types (used by code generator)
export interface InternalContract {
    name: string;
    user_variable_positions: VariablePosition[];
    user_variables_size: number;
    user_variables_depth: number;
    global_variable_positions: VariablePosition[];
    global_variables_size: number;
    global_variables_depth: number;
    functions: InternalFunction[];
    types: TypeDefinition[];
    structs: InternalStruct[];
}

export interface VariablePosition {
    name: string;
    offset: number | bigint;
    array_length: number | bigint;
    nth_size: number | bigint;
    typeId: number;
    children: VariablePosition[];
}

export interface InternalFunction {
    name: string;
    field_flat_paths: FieldPath[];
    fields_size: number;
    return_size: number;
    return_type_flat_paths: FieldPath[];
}

export interface FieldPath {
    path: string[];
    typeId: number;
}

export interface TypeDefinition {
    typeId: number;
    typeName: string;
}

export interface InternalStruct {
    name: string;
    fields: Array<{
        name: string;
        typeId: number;
    }>;
}
