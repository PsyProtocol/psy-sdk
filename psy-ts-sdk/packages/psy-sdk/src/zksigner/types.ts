import { DPNFunctionCircuitDefinition } from "../local-prover-rpc/types";
import { ContractCallArgs, JobInfo } from "../types";

type TPsyTransactionSignerAbility = "sign-hash" | "export-private-key-hex";
type TPsyTransactionSignerProviderAbility = "import-private-key" | "add-random-private-key";
interface IPsyTransactionSigner {
    getPublicKeyHex(): Promise<string>;
    getPrivateKeyHex?(): Promise<string>;
    getSignType?(): Promise<string>;
    getFingerprint?(): Promise<string|null|undefined>;
    // signHash?(hash: QHashOut): Promise<ProofWithPublicInputs>;
    signAndSubmit(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getAbilities(): TPsyTransactionSignerAbility[];
    registerUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}

interface IPsyTransactionSignerProvider {
    getSigners(): Promise<IPsyTransactionSigner[]>;
    getPublicKeysHex(): Promise<string[]>;
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IPsyTransactionSigner>;
    getAbilities(): TPsyTransactionSignerProviderAbility[];
    importPrivateKey?(privateKeyHex: string, signType: string, fingerprint?: string): Promise<IPsyTransactionSigner>;
    addRandomPrivateKey?(signType: string, fingerprint?: string): Promise<IPsyTransactionSigner>;
    registerUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}

export type {
    IPsyTransactionSigner,
    TPsyTransactionSignerAbility,
    IPsyTransactionSignerProvider,
    TPsyTransactionSignerProviderAbility,
};
