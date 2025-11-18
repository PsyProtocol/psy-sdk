export class TypesGenerator {
    generate(): string {
        return `// Auto-generated type definitions - Do not edit manually

// Common types used throughout the SDK
export type BigNumberish = bigint | string | number;
export type GUint = bigint | number;;
export type GHash = [GUint, GUint, GUint, GUint];

// Signer interface - holds public key for transaction signing
export interface ISigner {
  publicKey: string;
  provider: IContractProvider;
}

// Contract provider interface
export interface IContractProvider {
  getContractState(
    checkpointId: GUint,
    contractId: GUint,
    userId: GUint,
    slots: GUint[]
  ): Promise<GUint[]>;
  
  sendTransaction(
    contractId: GUint,
    functionName: string,
    args: any[],
    publicKey: string
  ): Promise<any>;
}

// Decodable interface for recursive decoding
export interface Decodable<T> {
  decode(data: GUint[]): T;
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
  attachTo<T>(ContractClass: new (checkpointId: GUint, userId: GUint, contractId: GUint, signer: ISigner) => T, checkpointId: GUint, userId: GUint, contractId: GUint): T {
    return new ContractClass(checkpointId, userId, contractId, this);
  }
}

export interface OtherUserInfo {
  amount_sent: GUint;
  amount_claimed: GUint;
}`;
    }
}
