import { ContractCallData, DPNFunctionCircuitDefinition, SignType } from "../local-prover-rpc/types";
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
