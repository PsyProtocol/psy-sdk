// packages/codegen/src/converters/abi-converter.ts

import { PsyJSON } from "@psy-protocol/psy-sdk";
import {
    AbiFormat,
    StructDefinition,
    FieldDefinition,
    ArrayType,
    InternalContract,
    VariablePosition,
    InternalFunction,
    TypeDefinition,
    InternalStruct,
    FieldPath,
} from "../types/abi-format";

// The AbiConverter class converts ABI to an internal representation.
export class AbiConverter {
    private typeRegistry = new Map<string, number>();
    private typeIdCounter = 0;

    // Convert the input ABI into version, contracts, and types.
    convert(abi: AbiFormat): { version: string; contracts: InternalContract[]; types: TypeDefinition[] } {
        // Find struct definitions marked as contracts.
        const contractStructs = abi.structs.filter((s) => s.is_contract);

        if (contractStructs.length === 0) {
            throw new Error("No contract found in ABI (no struct with is_contract: true)");
        }

        // First pass: register basic types and custom struct types.
        this.registerBasicTypes();
        this.registerStructTypes(abi.structs);

        // Convert all contract structs into internal representations.
        const contracts = contractStructs.map((contractStruct) => this.convertContract(contractStruct, abi.structs));

        // Generate the types array for export.
        const types = Array.from(this.typeRegistry.entries()).map(([typeName, typeId]) => ({
            typeId,
            typeName,
        }));

        return {
            version: abi.version,
            contracts,
            types,
        };
    }

    // Register basic types: Felt, Bool, Address.
    private registerBasicTypes() {
        this.typeRegistry.set("Felt", this.typeIdCounter++);
        this.typeRegistry.set("Bool", this.typeIdCounter++);
        //note: Our contract does not have the address type
        this.typeRegistry.set("Address", this.typeIdCounter++);
    }

    // Register struct types and array types found in fields.
    private registerStructTypes(structs: StructDefinition[]) {
        // Register all non-contract structs.
        for (const struct of structs) {
            if (!struct.is_contract && !this.typeRegistry.has(struct.name)) {
                this.typeRegistry.set(struct.name, this.typeIdCounter++);
            }
        }

        // Register array types found within struct fields.
        for (const struct of structs) {
            for (const field of struct.fields) {
                if (typeof field.type === "object" && field.type.type === "Array") {
                    const arrayTypeName = `${field.type.inner_type}[]`;
                    if (!this.typeRegistry.has(arrayTypeName)) {
                        this.typeRegistry.set(arrayTypeName, this.typeIdCounter++);
                    }
                }
            }
        }
    }

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

    // Convert a contract struct into an internal contract representation, including fields and functions.
    private convertContract(contractStruct: StructDefinition, allStructs: StructDefinition[]): InternalContract {
        // Convert fields into variable position descriptions.
        const { positions, totalSize, maxDepth } = this.convertFieldsToPositions(
            contractStruct.fields,
            allStructs,
            BigInt(0)
        );

        // Convert function descriptions.
        const functions = (contractStruct.functions || []).map((fn) => this.convertFunction(fn));

        // Convert all non-contract structs into internal struct representations.
        const structs = allStructs.filter((s) => !s.is_contract).map((s) => this.convertStruct(s));

        return {
            name: contractStruct.name,
            user_variable_positions: positions,
            user_variables_size: Number(totalSize),
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

    // Recursively convert a list of fields into position descriptions, compute total size and max depth.
    private convertFieldsToPositions(
        fields: FieldDefinition[],
        allStructs: StructDefinition[],
        baseOffset: bigint
    ): { positions: VariablePosition[]; totalSize: bigint; maxDepth: number } {
        const positions: VariablePosition[] = [];
        let currentOffset = baseOffset;
        let maxDepth = 0;

        for (const field of fields) {
            const { position, size, depth } = this.convertFieldToPosition(field, allStructs, currentOffset);

            positions.push(position);
            currentOffset += size;
            maxDepth = Math.max(maxDepth, depth);
        }

        return {
            positions,
            totalSize: currentOffset - baseOffset,
            maxDepth,
        };
    }

    // Convert a single field into a position description, handling simple types, structs, and arrays.
    private convertFieldToPosition(
        field: FieldDefinition,
        allStructs: StructDefinition[],
        offset: bigint
    ): { position: VariablePosition; size: bigint; depth: number } {
        if (typeof field.type === "string") {
            // Handle struct types.
            const struct = allStructs.find((s) => s.name === field.type && !s.is_contract);

            if (struct) {
                // Nested struct, recursively process its fields.
                const { positions, totalSize, maxDepth } = this.convertFieldsToPositions(
                    struct.fields,
                    allStructs,
                    BigInt(0)
                );

                return {
                    position: {
                        name: field.name,
                        offset,
                        array_length: 1,
                        nth_size: totalSize,
                        typeId: this.getTypeId(field.type),
                        children: positions,
                    },
                    size: totalSize,
                    depth: maxDepth + 1,
                };
            } else {
                // Simple types (e.g., Felt, Bool).
                return {
                    position: {
                        name: field.name,
                        offset,
                        array_length: 1,
                        nth_size: 0,
                        typeId: this.getTypeId(field.type),
                        children: [],
                    },
                    size: BigInt(1),
                    depth: 0,
                };
            }
        } else if (typeof field.type === "object" && field.type.type === "Array") {
            // Handle array types.
            const arrayType = field.type;
            const elementStruct = allStructs.find((s) => s.name === arrayType.inner_type && !s.is_contract);
            let elementSize: bigint;
            let elementChildren: VariablePosition[] = [];
            let elementDepth = 0;

            if (elementStruct) {
                // Array of structs.
                const { positions, totalSize, maxDepth } = this.convertFieldsToPositions(
                    elementStruct.fields,
                    allStructs,
                    BigInt(0)
                );
                elementSize = totalSize;
                elementChildren = positions;
                elementDepth = maxDepth;
            } else {
                // Array of simple types.
                elementSize = BigInt(1);
            }

            // Create a template position for array elements.
            const arrayElement: VariablePosition = {
                name: "[]",
                offset: 0,
                array_length: arrayType.length,
                nth_size: elementSize,
                typeId: this.getTypeId(arrayType.inner_type),
                children: elementChildren,
            };

            return {
                position: {
                    name: field.name,
                    offset,
                    array_length: arrayType.length,
                    nth_size: elementSize,
                    typeId: this.getTypeId(field.type),
                    children: [arrayElement],
                },
                size: BigInt(arrayType.length) * elementSize,
                depth: elementDepth + 1,
            };
        }

        throw new Error(`Unknown field type: ${PsyJSON.stringify(field.type)}`);
    }

    // Flatten function parameters and returns into field paths.
    private convertFunction(fn: any): InternalFunction {
        const fieldPaths: FieldPath[] = fn.params.map((param: any) => ({
            path: [param.name],
            typeId: this.getTypeId(param.type),
            type: param.type,
        }));

        const returnPaths: FieldPath[] = fn.return.map((ret: any) => ({
            path: ret.name ? [ret.name] : [],
            typeId: this.getTypeId(ret.type),
            type: ret.type,
        }));

        return {
            name: fn.name,
            field_flat_paths: fieldPaths,
            fields_size: fieldPaths.length,
            return_size: returnPaths.length,
            return_type_flat_paths: returnPaths,
        };
    }

    // Convert non-contract structs to internal struct types with name and type IDs only.
    private convertStruct(struct: StructDefinition): InternalStruct {
        return {
            name: struct.name,
            fields: struct.fields.map((field) => ({
                name: field.name,
                typeId: this.getTypeId(field.type),
                type: field.type,
            })),
        };
    }
}
