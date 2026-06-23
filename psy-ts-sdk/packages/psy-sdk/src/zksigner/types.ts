import { ClaimBatchItem, ContractCallData, DPNFunctionCircuitDefinition, GeneratedTxTraceJson, ProveTxTraceResumableJson, SignType, TxMetadata } from "../local-prover-rpc/types";
import { ContractCallArgs } from "../types";

type TPsyTransactionSignerAbility = "sign-hash" | "export-private-key-hex";
type TPsyTransactionSignerProviderAbility = "import-private-key" | "add-random-private-key";
interface IPsyTransactionSigner {
    getPublicKeyHex(): Promise<string>;
    getPrivateKeyHex?(): Promise<string>;
    getSignType?(): Promise<string>;
    getFingerprint?(): Promise<string|null|undefined>;
    // signHash?(hash: QHashOut): Promise<ProofWithPublicInputs>;
    signAndSubmit(pk_hash: string, callData: ContractCallData): Promise<string>;
    execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata>;
    // Decoupled exec/prove: produce a savable trace envelope (keyed by sig_hash) so the
    // wallet can persist it and prove/track it later. The SDK holds no state of its own.
    generateTxTrace(pk_hash: string, callData: ContractCallData): Promise<GeneratedTxTraceJson>;
    // Batch-claim variant: produces the same GeneratedTxTraceJson envelope, so the wallet
    // proves/tracks it via the shared proveTxTraceResumable below.
    generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTraceJson>;
    proveTxTraceResumable(pk_hash: string, envelope: string | GeneratedTxTraceJson): Promise<ProveTxTraceResumableJson>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getAbilities(): TPsyTransactionSignerAbility[];
    registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}

interface IPsyTransactionSignerProvider {
    getSigners(): Promise<IPsyTransactionSigner[]>;
    getPublicKeysHex(): Promise<string[]>;
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IPsyTransactionSigner>;
    getAbilities(): TPsyTransactionSignerProviderAbility[];
    importPrivateKey?(privateKeyHex: string, signType: SignType, fingerprint: string): Promise<IPsyTransactionSigner>;
    addRandomPrivateKey?(signType: SignType): Promise<IPsyTransactionSigner>;
    registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}

export type {
    IPsyTransactionSigner,
    TPsyTransactionSignerAbility,
    IPsyTransactionSignerProvider,
    TPsyTransactionSignerProviderAbility,
};
