// packages/codegen/src/converters/abi-converter.ts

import { PsyJSON } from "@psy-protocol/psy-sdk";
import {
    AbiInput,
    Abi,
    AbiMethod,
    AbiParam,
    AbiStateField,
    AbiStructType,
    ArrayType,
    InternalContract,
    VariablePosition,
    InternalFunction,
    TypeDefinition,
    InternalStruct,
    FieldPath,
    TypeRef,
} from "../types/abi-format";

// The AbiConverter class converts ABI to an internal representation.
// Accepts the ABI (`{ schema_version, contract, types[] }`).
export class AbiConverter {
    private typeRegistry = new Map<string, number>();
    private typeIdCounter = 0;
    private static readonly pseudoMapStructNames = new Set(["Map", "ContractHashMap", "NamespacedMap"]);

    // Convert the ABI into version, contracts, and types.
    convert(abi: AbiInput): { version: string; contracts: InternalContract[]; types: TypeDefinition[] } {
        return this.convertAbi(abi);
    }

    // -------------------------------------------------------------------------
    // ABI conversion path
    // -------------------------------------------------------------------------

    // Convert the ABI (`{ schema_version, contract, types[] }`) into the
    // internal contract layout. Offsets are read directly from
    // `contract.state[].offset` (already computed by the compiler), and
    // `state_mutability` is taken explicitly.
    private convertAbi(abi: Abi): { version: string; contracts: InternalContract[]; types: TypeDefinition[] } {
        // First pass: register basic types, named struct types from types[],
        // and array types encountered in struct fields, state fields, and
        // method params/returns.
        this.registerBasicTypes();
        this.registerAbiStructTypes(abi.types);
        for (const field of abi.contract.state) {
            this.registerAbiArrayTypes(field.type);
        }
        for (const method of abi.contract.methods) {
            for (const param of method.inputs) {
                this.registerAbiArrayTypes(param.type);
            }
            for (const param of method.outputs) {
                this.registerAbiArrayTypes(param.type);
            }
        }

        // Convert the single contract into its internal representation.
        const contract = this.convertAbiContract(abi.contract, abi.types);

        const types = Array.from(this.typeRegistry.entries()).map(([typeName, typeId]) => ({
            typeId,
            typeName,
        }));

        return {
            version: abi.schema_version,
            contracts: [contract],
            types,
        };
    }

    // Register named struct types from the `types[]` table, plus array
    // types encountered in struct fields and state.
    private registerAbiStructTypes(types: AbiStructType[]) {
        for (const ty of types) {
            if (!this.typeRegistry.has(ty.name)) {
                this.typeRegistry.set(ty.name, this.typeIdCounter++);
            }
        }
        // Register array types found in struct fields.
        for (const ty of types) {
            for (const field of ty.fields) {
                this.registerAbiArrayTypes(field.type);
            }
        }
    }

    // Recursively register array types referenced by a TypeRef.
    private registerAbiArrayTypes(ref: TypeRef): void {
        if (ref.kind === "array") {
            const innerName = this.abiItemTypeName(ref.item);
            const arrayTypeName = `${innerName}[]`;
            if (!this.typeRegistry.has(arrayTypeName)) {
                // Ensure the inner type is registered first.
                this.ensureAbiTypeRegistered(ref.item);
                this.typeRegistry.set(arrayTypeName, this.typeIdCounter++);
            }
            this.registerAbiArrayTypes(ref.item);
        } else if (ref.kind === "struct") {
            // Struct types are registered by registerAbiStructTypes.
        } else if (ref.kind === "map") {
            this.registerAbiArrayTypes(ref.key);
            this.registerAbiArrayTypes(ref.value);
        }
    }

    // Ensure a TypeRef's named type is present in the registry.
    private ensureAbiTypeRegistered(ref: TypeRef): void {
        if (ref.kind === "struct") {
            if (!this.typeRegistry.has(ref.name)) {
                this.typeRegistry.set(ref.name, this.typeIdCounter++);
            }
        } else if (ref.kind === "primitive") {
            const name = this.primitiveToTypeName(ref.name);
            if (!this.typeRegistry.has(name)) {
                this.typeRegistry.set(name, this.typeIdCounter++);
            }
        }
    }

    // Map a primitive name to the type name used by the
    // internal type registry and serializer (which understands Felt/Bool/etc.).
    private primitiveToTypeName(name: string): string {
        switch (name) {
            case "Felt":
                return "Felt";
            case "Bool":
                return "Bool";
            case "U32":
                return "u32";
            case "Hash":
                return "Hash";
            default:
                return name;
        }
    }

    // Resolve the item type name for an array element TypeRef.
    // Array items are only ever primitives or structs in the schema,
    // but we handle the other variants defensively.
    private abiItemTypeName(ref: TypeRef): string {
        switch (ref.kind) {
            case "primitive":
                return this.primitiveToTypeName(ref.name);
            case "struct":
                return ref.name;
            case "array":
                return `${this.abiItemTypeName(ref.item)}[]`;
            case "map":
                return this.mapKindToString(ref.map_kind);
        }
    }

    // Convert the contract section into an internal contract.
    private convertAbiContract(
        contract: Abi["contract"],
        types: AbiStructType[]
    ): InternalContract {
        // Convert state fields into positions, reading offsets directly from
        // `contract.state[].offset` (already absolute, compiler-computed).
        const positions = contract.state.map((field) => this.convertAbiStateField(field, types));

        // Compute total state size as the sum of felt_size across state fields.
        const totalSize = contract.state.reduce((sum, f) => sum + f.felt_size, 0);

        // Compute max depth across state positions.
        const maxDepth = positions.reduce((max, p) => Math.max(max, this.positionDepth(p)), 0);

        // Convert methods into internal functions, carrying method_id and
        // state_mutability explicitly.
        const functions = contract.methods.map((m) => this.convertAbiMethod(m));

        // Convert all named struct types into internal struct representations.
        const structs = types
            .filter((t) => !this.isPseudoMapStruct(t))
            .map((t) => this.convertAbiStruct(t));

        return {
            name: contract.name,
            user_variable_positions: positions,
            user_variables_size: totalSize,
            user_variables_depth: maxDepth,
            global_variable_positions: [],
            global_variables_size: 0,
            global_variables_depth: 0,
            functions,
            types: Array.from(this.typeRegistry.entries()).map(([typeName, typeId]) => ({
                typeId,
                typeName,
            })),
            structs,
        };
    }

    // Convert a state field into a variable position, reading the
    // absolute offset directly. Nested struct/array children are resolved from
    // the `types[]` table.
    private convertAbiStateField(field: AbiStateField, types: AbiStructType[]): VariablePosition {
        const ref = field.type;
        const offset = BigInt(field.offset);

        if (ref.kind === "primitive") {
            const typeName = this.primitiveToTypeName(ref.name);
            return {
                name: field.name,
                offset,
                array_length: 1,
                nth_size: BigInt(field.felt_size),
                typeId: this.getTypeId(typeName),
                children: [],
            };
        }

        if (ref.kind === "struct") {
            const structType = types.find((t) => t.name === ref.name);
            if (!structType) {
                throw new Error(`Unknown struct type in state: ${ref.name}`);
            }
            const children = this.convertAbiStructFields(structType, types);
            return {
                name: field.name,
                offset,
                array_length: 1,
                nth_size: BigInt(structType.felt_size),
                typeId: this.getTypeId(ref.name),
                children,
            };
        }

        if (ref.kind === "array") {
            const itemRef = ref.item;
        let elementSize = BigInt(ref.item_felt_size);
        let elementChildren: VariablePosition[] = [];

        if (itemRef.kind === "struct") {
            const structType = types.find((t) => t.name === itemRef.name);
            if (!structType) {
                throw new Error(`Unknown struct type in array item: ${itemRef.name}`);
            }
            elementChildren = this.convertAbiStructFields(structType, types);
        }

            const innerTypeName = this.abiItemTypeName(itemRef);
            const arrayElement: VariablePosition = {
                name: "[]",
                offset: 0,
                array_length: ref.length,
                nth_size: elementSize,
                typeId: this.getTypeId(innerTypeName),
                children: elementChildren,
            };

            const arrayTypeName = `${innerTypeName}[]`;
            return {
                name: field.name,
                offset,
                array_length: ref.length,
                nth_size: elementSize,
                typeId: this.getTypeId(arrayTypeName),
                children: [arrayElement],
            };
        }

        // Map types are modeled as virtual-region storage with a felt footprint.
        // The runtime state proxy treats them as opaque regions; offset is the
        // compiler-emitted base offset and felt_size is the total footprint.
        if (ref.kind === "map") {
            return {
                name: field.name,
                offset,
                array_length: 1,
                nth_size: 0, // primitive: runtime proxy returns opaque hash felts
                typeId: this.typeIdCounter++, // opaque region; no shared type id
                children: [],
            };
        }

        throw new Error(`Unknown TypeRef kind in state: ${(ref as TypeRef).kind}`);
    }

    // Convert a struct's fields into child variable positions. Each
    // field's offset is `offset_within_parent` (relative to the parent struct's
    // base offset, which the caller adds when traversing).
    private convertAbiStructFields(structType: AbiStructType, types: AbiStructType[]): VariablePosition[] {
        return structType.fields.map((field) => {
            const ref = field.type;
            const relOffset = BigInt(field.offset_within_parent);

            if (ref.kind === "primitive") {
                const typeName = this.primitiveToTypeName(ref.name);
                return {
                    name: field.name,
                    offset: relOffset,
                    array_length: 1,
                    nth_size: BigInt(field.felt_size),
                    typeId: this.getTypeId(typeName),
                    children: [],
                };
            }

            if (ref.kind === "struct") {
                const nestedType = types.find((t) => t.name === ref.name);
                if (!nestedType) {
                    throw new Error(`Unknown nested struct type: ${ref.name}`);
                }
                const children = this.convertAbiStructFields(nestedType, types);
                return {
                    name: field.name,
                    offset: relOffset,
                    array_length: 1,
                    nth_size: BigInt(nestedType.felt_size),
                    typeId: this.getTypeId(ref.name),
                    children,
                };
            }

            if (ref.kind === "array") {
                const itemRef = ref.item;
                let elementChildren: VariablePosition[] = [];
                if (itemRef.kind === "struct") {
                    const nestedType = types.find((t) => t.name === itemRef.name);
                    if (!nestedType) {
                        throw new Error(`Unknown struct type in array item: ${itemRef.name}`);
                    }
                    elementChildren = this.convertAbiStructFields(nestedType, types);
                }
                const innerTypeName = this.abiItemTypeName(itemRef);
                const arrayElement: VariablePosition = {
                    name: "[]",
                    offset: 0,
                    array_length: ref.length,
                    nth_size: BigInt(ref.item_felt_size),
                    typeId: this.getTypeId(innerTypeName),
                    children: elementChildren,
                };
                const arrayTypeName = `${innerTypeName}[]`;
                return {
                    name: field.name,
                    offset: relOffset,
                    array_length: ref.length,
                    nth_size: BigInt(ref.item_felt_size),
                    typeId: this.getTypeId(arrayTypeName),
                    children: [arrayElement],
                };
            }

            // Map-typed struct field: opaque region.
            if (ref.kind === "map") {
                return {
                    name: field.name,
                    offset: relOffset,
                    array_length: 1,
                    nth_size: 0, // primitive: opaque hash felts
                    typeId: this.typeIdCounter++,
                    children: [],
                };
            }

            throw new Error(`Unknown TypeRef kind in struct field: ${(ref as TypeRef).kind}`);
        });
    }

    // Convert a method into an internal function, carrying
    // method_id and state_mutability explicitly (no naming heuristic).
    private convertAbiMethod(method: AbiMethod): InternalFunction {
        const fieldPaths: FieldPath[] = method.inputs.map((param: AbiParam) => ({
            path: [param.name],
            typeId: this.getTypeId(this.typeRefToLegacyName(param.type)),
            type: this.typeRefToLegacyType(param.type),
        }));

        const returnPaths: FieldPath[] = method.outputs.map((ret: AbiParam) => ({
            path: [ret.name],
            typeId: this.getTypeId(this.typeRefToLegacyName(ret.type)),
            type: this.typeRefToLegacyType(ret.type),
        }));

        return {
            name: method.name,
            field_flat_paths: fieldPaths,
            fields_size: method.input_felt_count,
            return_size: method.output_felt_count,
            return_type_flat_paths: returnPaths,
            method_id: method.method_id,
            state_mutability: method.state_mutability,
        };
    }

    // Convert a struct type into an internal struct, mapping each
    // field's TypeRef to the legacy `string | ArrayType` representation the
    // serializer and generator consume.
    private convertAbiStruct(struct: AbiStructType): InternalStruct {
        return {
            name: struct.name,
            fields: struct.fields.map((field) => {
                const typeName = this.typeRefToLegacyName(field.type);
                // Map-typed struct fields produce a composite type name (e.g.
                // "ContractHashMap<Hash, Hash, 1024>") that is never pre-registered.
                // Assign an opaque type id instead of throwing.
                if (!this.typeRegistry.has(typeName)) {
                    this.typeRegistry.set(typeName, this.typeIdCounter++);
                }
                return {
                    name: field.name,
                    typeId: this.getTypeId(typeName),
                    type: this.typeRefToLegacyType(field.type),
                    felt_size: field.felt_size,
                };
            }),
        };
    }

    // Map a TypeRef to the legacy type-name string used by the type registry
    // (primitives become Felt/Bool/u32/Hash; structs keep their name).
    private typeRefToLegacyName(ref: TypeRef): string {
        switch (ref.kind) {
            case "primitive":
                return this.primitiveToTypeName(ref.name);
            case "struct":
                return ref.name;
            case "array":
                return `${this.typeRefToLegacyName(ref.item)}[]`;
            case "map": {
                const kindStr = this.mapKindToString(ref.map_kind);
                return `${kindStr}<${this.typeRefToLegacyName(ref.key)}, ${this.typeRefToLegacyName(ref.value)}, ${ref.capacity}>`;
            }
        }
    }

    // Map a TypeRef to the legacy `string | ArrayType` representation the
    // serializer and TS-type generator consume. Struct and primitive refs become
    // their name string; array refs become the `{ type: "Array", inner_type, length }`
    // object; map refs stringify to their map-kind name (matching the compiler's
    // legacy spec adapter, which has no map representation).
    private typeRefToLegacyType(ref: TypeRef): string | ArrayType {
        switch (ref.kind) {
            case "primitive":
                return this.primitiveToTypeName(ref.name);
            case "struct":
                return ref.name;
            case "array":
                return {
                    type: "Array",
                    inner_type: this.abiItemTypeName(ref.item),
                    length: ref.length,
                };
            case "map":
                return this.mapKindToString(ref.map_kind);
        }
    }

    private mapKindToString(mapKind: string): string {
        switch (mapKind) {
            case "contract_hash_map":
                return "ContractHashMap";
            case "map":
                return "Map";
            case "namespaced_map":
                return "NamespacedMap";
            default:
                return mapKind;
        }
    }

    private isPseudoMapStruct(struct: AbiStructType): boolean {
        return struct.felt_size === 0 && struct.fields.length === 0 && AbiConverter.pseudoMapStructNames.has(struct.name);
    }

    // Compute the depth of a variable position tree.
    private positionDepth(pos: VariablePosition): number {
        if (pos.children.length === 0) return 0;
        let max = 0;
        for (const child of pos.children) {
            max = Math.max(max, this.positionDepth(child));
        }
        return max + 1;
    }

    // Register basic types used by legacy and ABI formats.
    private registerBasicTypes() {
        this.typeRegistry.set("Felt", this.typeIdCounter++);
        this.typeRegistry.set("Bool", this.typeIdCounter++);
        this.typeRegistry.set("u32", this.typeIdCounter++);
        this.typeRegistry.set("Hash", this.typeIdCounter++);
        // note: Our contract does not have the address type
        this.typeRegistry.set("Address", this.typeIdCounter++);
    }

    // Register struct types and array types found in fields.
    // Get the type ID for simple or array types.
    private getTypeId(type: string | ArrayType): number {
        if (typeof type === "string") {
            const typeId = this.typeRegistry.get(type);
            if (typeId === undefined) {
                throw new Error(`Unknown type: ${type}`);
            }
            return typeId;
        } else if (type.type === "Array") {
            const arrayTypeName = `${type.inner_type}[]`;
            let typeId = this.typeRegistry.get(arrayTypeName);
            // If array type is not registered, automatically register it
            if (typeId === undefined) {
                // First ensure the inner type is registered
                this.getTypeId(type.inner_type);
                // Then register the array type
                typeId = this.typeIdCounter++;
                this.typeRegistry.set(arrayTypeName, typeId);
            }
            return typeId;
        }
        throw new Error(`Invalid type: ${PsyJSON.stringify(type)}`);
    }
}
