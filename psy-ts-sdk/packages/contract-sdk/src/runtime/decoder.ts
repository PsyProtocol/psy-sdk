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
}
