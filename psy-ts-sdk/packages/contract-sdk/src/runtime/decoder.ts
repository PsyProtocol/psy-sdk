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

        // Multiple paths: could be a struct or array
        // Try to detect if it's a struct (paths have non-numeric names)
        const isStruct = returnTypePaths.every(p =>
            p.path.length > 0 && isNaN(Number(p.path[p.path.length - 1]))
        );

        if (isStruct) {
            // Reconstruct struct from fields
            const result: any = {};
            returnTypePaths.forEach((pathInfo, index) => {
                const fieldName = pathInfo.path[pathInfo.path.length - 1];
                result[fieldName] = this.decodeField(pathInfo, index, rawFelts, structs);
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

        // Check if it's a known struct type
        const structDef = structs.get(typeName);
        if (structDef) {
            const result: any = {};
            structDef.fields.forEach((field, fieldIdx) => {
                const fieldPath = [...pathInfo.path, field.name];
                result[field.name] = this.decode(field.name, fieldPath, rawFelts[index + fieldIdx] ?? BigInt(0));
            });
            return result;
        }

        // Use the type name for decoding
        const decoder = this.decoders.get(typeName);
        if (decoder) {
            return decoder(rawValue);
        }

        // Check type name for primitive type hints
        const lowerType = typeName.toLowerCase();
        if (lowerType === "bool") {
            return rawValue !== BigInt(0);
        }
        if (lowerType === "felt" || lowerType === "u32" || lowerType === "hash") {
            return BigInt(rawValue);
        }

        return BigInt(rawValue);
    }
}
