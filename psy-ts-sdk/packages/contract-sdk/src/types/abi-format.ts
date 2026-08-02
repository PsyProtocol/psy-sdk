// packages/codegen/src/types/abi-format.ts

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
    // ABI metadata.
    method_id?: number;
    state_mutability?: StateMutability;
}

export interface FieldPath {
    path: string[];
    typeId: number;
    type?: string | ArrayType;
}

export interface TypeDefinition {
    typeId: number;
    typeName: string;
}

export interface ArrayType {
    type: "Array";
    inner_type: string;
    length: number;
}

export interface InternalStruct {
    name: string;
    fields: Array<{
        name: string;
        typeId: number;
        type?: string | ArrayType; // Added type field to support complete type information
        felt_size?: number; // Preserves zero-sized opaque map fields.
    }>;
}

// ---------------------------------------------------------------------------
// ABI types — the contract artifact emitted by the compiler.
// ---------------------------------------------------------------------------

/** Method mutability. */
export type StateMutability = "view" | "external";

/** The set of primitive type names the compiler emits. */
export type PrimitiveTypeName = "Felt" | "Bool" | "U32" | "Hash";

/** Source-level map type family. */
export type MapKind = "contract_hash_map" | "map" | "namespaced_map";

/** Recursive type reference used by state, params, and struct fields. */
export type TypeRef =
    | { kind: "primitive"; name: PrimitiveTypeName }
    | { kind: "struct"; name: string }
    | { kind: "array"; item: TypeRef; length: number; item_felt_size: number }
    | {
          kind: "map";
          map_kind: MapKind;
          key: TypeRef;
          value: TypeRef;
          capacity: number;
          value_felt_size: number;
          alignment_felts: number;
      };

/** A named struct type entry in the `types[]` table. */
export interface AbiStructType {
    kind: "struct";
    name: string;
    felt_size: number;
    fields: AbiStructField[];
}

/** A field within an `AbiStructType`. */
export interface AbiStructField {
    name: string;
    type: TypeRef;
    offset_within_parent: number;
    felt_size: number;
}

/** A state field with its absolute slot offset and felt footprint. */
export interface AbiStateField {
    name: string;
    type: TypeRef;
    offset: number;
    felt_size: number;
}

/** A typed parameter (input or output). */
export interface AbiParam {
    name: string;
    type: TypeRef;
    felt_size: number;
}

/** A method with explicit compiler-owned metadata. */
export interface AbiMethod {
    name: string;
    method_id: number;
    state_mutability: StateMutability;
    inputs: AbiParam[];
    outputs: AbiParam[];
    input_felt_count: number;
    output_felt_count: number;
    vm_type?: string;
}

/** Contract-level metadata + state layout + methods. */
export interface AbiContract {
    name: string;
    state_tree_height: number;
    state: AbiStateField[];
    methods: AbiMethod[];
}

/** Top-level ABI shape. */
export interface Abi {
    schema_version: string;
    contract: AbiContract;
    types: AbiStructType[];
}

// ---------------------------------------------------------------------------
// Accepted ABI input.
// ---------------------------------------------------------------------------

/** Accepted ABI input. */
export type AbiInput = Abi;

/**
 * Type guard: `true` when the input is the compiler-emitted ABI payload.
 *
 * The ABI carries a top-level `contract` object and a `schema_version`
 * string; the struct-format ABI carries a top-level `structs` array and a
 * `version` string.
 */
export function isAbi(abi: AbiInput): abi is Abi {
    return (
        typeof abi === "object" &&
        abi !== null &&
        "contract" in abi &&
        typeof (abi as Abi).contract === "object" &&
        (abi as Abi).contract !== null &&
        "schema_version" in abi
    );
}