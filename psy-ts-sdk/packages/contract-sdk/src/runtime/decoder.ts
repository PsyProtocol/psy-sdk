// Felt type is used implicitly via any/BigInt

export class RecursiveDecoder {
    private decoders: Map<string, (data: any) => any> = new Map();

    constructor() {
        this.registerDefaultDecoders();
    }

    private registerDefaultDecoders(): void {
        this.decoders.set("uint256", (data) => BigInt(data));
        this.decoders.set("Felt", (data) => BigInt(data));
        this.decoders.set("bool", (data) => data !== BigInt(0));
        this.decoders.set("address", (data) => {
            const addr = data.toString(16).padStart(40, "0");
            return "0x" + addr;
        });
        this.decoders.set("array", (data) => {
            if (Array.isArray(data)) {
                return data.map((item) => this.decode("element", [], item));
            }
            return [];
        });
    }

    decode(varName: string, path: string[], rawValue: any): any {
        const type = this.inferType(varName, path);
        const decoder = this.decoders.get(type);

        if (decoder) {
            return decoder(rawValue);
        }

        return BigInt(rawValue);
    }

    decodeReturnValue(rawValue: any): any {
        if (Array.isArray(rawValue)) {
            return rawValue.map((v) => this.decode("return", [], v));
        }
        return this.decode("return", [], rawValue);
    }

    private inferType(varName: string, path: string[]): string {
        const fullPath = [varName, ...path].join(".");

        if (fullPath.includes("balance") || fullPath.includes("amount") || fullPath.includes("value")) {
            return "uint256";
        }

        if (fullPath.includes("address") || fullPath.includes("addr") || fullPath.includes("owner")) {
            return "address";
        }

        if (fullPath.includes("is_") || fullPath.includes("has_") || fullPath.includes("enabled")) {
            return "bool";
        }

        if (path.some((p) => !isNaN(Number(p)))) {
            return "array";
        }

        return "uint256";
    }

    registerDecoder(typeName: string, decoder: (data: any) => any): void {
        this.decoders.set(typeName, decoder);
    }

    registerStructDecoder<T>(
        structName: string,
        fields: { name: string; type: string }[]
    ): void {
        this.decoders.set(structName, (data: any[]) => {
            const result: any = {};
            fields.forEach((field, index) => {
                result[field.name] = this.decode(field.name, [], data[index]);
            });
            return result as T;
        });
    }

    /**
     * Decode return values using ABI return type paths.
     * Reconstructs structs/arrays from flat felt arrays using return_type_flat_paths.
     *
     * @param rawFelts - Raw felt array from contract execution result
     * @param returnTypePaths - Flat paths describing the return type structure
     * @param structs - Known struct definitions for type resolution
     */
    decodeReturnType(
        rawFelts: any[],
        returnTypePaths: { path: string[]; type?: string | any; typeId: number }[] | undefined,
        structs: Map<string, { name: string; fields: any[] }>
    ): any {
        if (!returnTypePaths || returnTypePaths.length === 0) {
            // No type info: fall back to basic decoding
            return this.decodeReturnValue(rawFelts);
        }

        // Check if the return type is a single primitive (1 path, 1 felt)
        if (returnTypePaths.length === 1) {
            const path = returnTypePaths[0];
            return this.decodeField(path, 0, rawFelts, structs);
        }

        // Multiple top-level return paths represent a named tuple/result object.
        // Do not infer this from field names: numeric field names are legal and
        // arrays are represented by a single top-level path with an Array type.
        const isNamedTuple = returnTypePaths.every(p => p.path.length === 1);

        if (isNamedTuple) {
            // Reconstruct named return object from fields
            const result: any = {};
            let offset = 0;
            returnTypePaths.forEach((pathInfo) => {
                const fieldName = pathInfo.path[pathInfo.path.length - 1];
                result[fieldName] = this.decodeField(pathInfo, offset, rawFelts, structs);
                offset += this.typeFeltSize(pathInfo.type, structs);
            });
            return result;
        }

        // Otherwise treat as array
        return rawFelts.map((felt, index) => {
            if (index < returnTypePaths.length) {
                return this.decodeField(returnTypePaths[index], index, rawFelts, structs);
            }
            return BigInt(felt);
        });
    }

    private decodeField(
        pathInfo: { path: string[]; type?: string | any; typeId: number },
        index: number,
        rawFelts: any[],
        structs: Map<string, { name: string; fields: any[] }>
    ): any {
        const rawValue = rawFelts[index] ?? BigInt(0);
        const typeName = pathInfo.type;

        // Handle array types: type is { type: "Array", inner_type, length }
        if (typeof typeName === "object" && typeName !== null) {
            // Array type: decode each element
            if (typeName.type === "Array") {
                const innerType = typeof typeName.inner_type === "string" ? typeName.inner_type : "Felt";
                const length = typeName.length ?? rawFelts.length - index;
                const result: any[] = [];
                for (let i = 0; i < length; i++) {
                    // Try struct decode for inner_type
                    const structDef = structs.get(innerType);
                    if (structDef) {
                        const elementOffset = index + i * this.structFeltSize(structDef, structs);
                        result.push(this.decodeStruct(structDef, elementOffset, rawFelts, structs, [...pathInfo.path, String(i)]));
                    } else {
                        const elemFelt = rawFelts[index + i] ?? BigInt(0);
                        result.push(BigInt(elemFelt));
                    }
                }
                return result;
            }
            // Unknown object type: fall through to BigInt
            return BigInt(rawValue);
        }

        // typeName is a string from here on
        const typeStr = typeName as string;

        // Check if it's a known struct type
        const structDef = structs.get(typeStr);
        if (structDef) {
            return this.decodeStruct(structDef, index, rawFelts, structs, pathInfo.path);
        }

        // Use the type name for decoding
        const decoder = this.decoders.get(typeStr);
        if (decoder) {
            return decoder(rawValue);
        }

        // Check type name for primitive type hints
        const lowerType = typeStr.toLowerCase();
        if (lowerType === "bool") {
            return rawValue !== BigInt(0);
        }
        if (lowerType === "felt" || lowerType === "u32" || lowerType === "hash") {
            return BigInt(rawValue);
        }

        return BigInt(rawValue);
    }

    private decodeStruct(
        structDef: { name: string; fields: any[] },
        startIndex: number,
        rawFelts: any[],
        structs: Map<string, { name: string; fields: any[] }>,
        path: string[]
    ): any {
        const result: any = {};
        let offset = startIndex;
        for (const field of structDef.fields) {
            const fieldPath = [...path, field.name];
            result[field.name] = this.decodeField(
                { path: fieldPath, type: field.type, typeId: field.typeId },
                offset,
                rawFelts,
                structs
            );
            offset += this.typeFeltSize(field.type, structs, field.felt_size);
        }
        return result;
    }

    private typeFeltSize(
        type: string | any | undefined,
        structs: Map<string, { name: string; fields: any[] }>,
        explicitSize?: number
    ): number {
        if (typeof explicitSize === "number" && Number.isFinite(explicitSize) && explicitSize > 0) {
            return explicitSize;
        }
        if (typeof type === "string") {
            const structDef = structs.get(type);
            return structDef ? this.structFeltSize(structDef, structs) : 1;
        }
        if (type && typeof type === "object" && type.type === "Array") {
            const length = typeof type.length === "number" ? type.length : 0;
            const innerSize = this.typeFeltSize(type.inner_type, structs);
            return Math.max(0, length) * innerSize;
        }
        return 1;
    }

    private structFeltSize(
        structDef: { name: string; fields: any[] },
        structs: Map<string, { name: string; fields: any[] }>
    ): number {
        return structDef.fields.reduce((sum, field) => sum + this.typeFeltSize(field.type, structs, field.felt_size), 0);
    }
}
