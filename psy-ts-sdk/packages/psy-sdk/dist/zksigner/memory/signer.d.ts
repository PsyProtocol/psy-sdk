import { NetworkId } from "../../action";
import { ContractCallArgs, ContractCallData, DPNFunctionCircuitDefinition, IPsyUserProverProvider, SignType, TxMetadata } from "../../local-prover-rpc";
import { IPsyTransactionSigner, TPsyTransactionSignerAbility } from "../types";
declare class PsyMemoryTransactionSigner implements IPsyTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    signType: string;
    fingerprint: string;
    prover: IPsyUserProverProvider;
    private constructor();
    static create(proverProvider: IPsyUserProverProvider, networkId: NetworkId, privateKeyHex: string, signType: SignType, fingerprint: string): Promise<PsyMemoryTransactionSigner>;
    getPrivateKeyHex(): Promise<string>;
    getSignType(): Promise<string>;
    getFingerprint(): Promise<string>;
    signAndSubmit(pk_hash: string, callData: ContractCallData): Promise<string>;
    execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getAbilities(): TPsyTransactionSignerAbility[];
    getPublicKeyHex(): Promise<string>;
    registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}
export { PsyMemoryTransactionSigner };
//# sourceMappingURL=signer.d.ts.map