export class TypesGenerator {
    generate(): string {
        return `// Auto-generated type definitions - Do not edit manually
import { IContractProvider } from '@psy-protocol/psy-sdk';

// Common types used throughout the SDK
export type Felt = bigint | number;
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

Array.prototype.toFelts = function <T>(): Felt[] {
  const felts: Felt[] = [];
  
  (this as T[]).forEach(item => {
    if (item && typeof (item as ToFelts).toFelts === "function") {
      felts.push(...(item as ToFelts).toFelts());
    } 
    else if (Array.isArray(item)) {
      felts.push(...(item as PsyFixedArray<any, any>).toFelts());
    } 
    else {
      felts.push(item as Felt);
    }
  });

  return felts;
};

// Signer interface - holds public key for transaction signing
export interface ISigner {
  publicKey: string;
  provider: IContractProvider;
}

// Re-export IContractProvider from psy-sdk
export type { IContractProvider };

// Decodable interface for recursive decoding
export interface Decodable<T> {
  decode(data: Felt[]): T;
}

// Standard Signer implementation
export class Signer implements ISigner {
  constructor(
    public publicKey: string,
    public provider: IContractProvider
  ) {}

  static fromPublicKey(publicKey: string, provider: IContractProvider): Signer {
    return new Signer(publicKey, provider);
  }

  // Convenience method to attach to a contract
  attachTo<T>(ContractClass: new (checkpointId: Felt, userId: Felt, contractId: Felt, signer: ISigner) => T, checkpointId: Felt, userId: Felt, contractId: Felt): T {
    return new ContractClass(checkpointId, userId, contractId, this);
  }
}`;
    }
}
