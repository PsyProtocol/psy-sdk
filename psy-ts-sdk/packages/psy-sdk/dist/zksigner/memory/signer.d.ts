import { NetworkId } from "../../action";
import { ClaimBatchItem, ContractCallArgs, ContractCallData, DPNFunctionCircuitDefinition, GeneratedTxTraceJson, IPsyUserProverProvider, ProveTxTraceResumableJson, SignType, TxMetadata } from "../../local-prover-rpc";
import { IPsyTransactionSigner, TPsyTransactionSignerAbility } from "../types";
declare class PsyMemoryTransactionSigner implements IPsyTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    signType: SignType;
    fingerprint: string;
    prover: IPsyUserProverProvider;
    private constructor();
    static create(proverProvider: IPsyUserProverProvider, networkId: NetworkId, privateKeyHex: string, signType: SignType, fingerprint: string): Promise<PsyMemoryTransactionSigner>;
    getPrivateKeyHex(): Promise<string>;
    getSignType(): Promise<string>;
    getFingerprint(): Promise<string>;
    signAndSubmit(pk_hash: string, callData: ContractCallData): Promise<string>;
    execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata>;
    generateTxTrace(pk_hash: string, callData: ContractCallData): Promise<GeneratedTxTraceJson>;
    generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTraceJson>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getAbilities(): TPsyTransactionSignerAbility[];
    getPublicKeyHex(): Promise<string>;
    registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
    proveTxTraceStep(pkHash: string, envelope: string | GeneratedTxTraceJson, resumeFrom?: {
        proof_tree_meta: unknown;
        last_step_info: unknown;
        current_header: unknown;
        previous_header: unknown;
        proof_blobs: Uint8Array[];
        next_step_index: number;
    }): Promise<ProveTxTraceResumableJson>;
}
export { PsyMemoryTransactionSigner };
//# sourceMappingURL=signer.d.ts.map