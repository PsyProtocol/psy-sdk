// Runtime type definitions
import { IContractProvider, IContractStateReader } from "@psy-protocol/psy-sdk";

export type Felt = bigint | number;
export type u32 = Felt;
export type GHash = [Felt, Felt, Felt, Felt];
export type PsyFixedArray<T, L extends number> = ReadonlyArray<T> & { length: L };

export interface ToFelts {
    toFelts(): Felt[];
}

export class FeltValue implements ToFelts {
    constructor(private value: Felt) {}

    toFelts(): Felt[] {
        return [this.value];
    }

    getValue(): Felt {
        return this.value;
    }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
(Array.prototype as any).toFelts = function <T>(): Felt[] {
    const felts: Felt[] = [];

    (this as T[]).forEach((item) => {
        if (item && typeof (item as ToFelts).toFelts === "function") {
            felts.push(...(item as ToFelts).toFelts());
        } else if (Array.isArray(item)) {
            felts.push(...(item as any).toFelts());
        } else {
            felts.push(item as Felt);
        }
    });

    return felts;
};

export interface ISigner {
    publicKey: string;
    provider: IContractProvider;
}

export type { IContractProvider, IContractStateReader };

export interface Decodable<T> {
    decode(data: Felt[]): T;
}

export class Signer implements ISigner {
    constructor(
        public publicKey: string,
        public provider: IContractProvider
    ) {}

    static fromPublicKey(publicKey: string, provider: IContractProvider): Signer {
        return new Signer(publicKey, provider);
    }
}
