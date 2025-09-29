import { DPNFunctionCircuitDefinition } from "../local-prover-rpc/types";
import { ContractCallArgs, JobInfo } from "../types";

type TQedTransactionSignerAbility = "sign-hash" | "export-private-key-hex";
type TQedTransactionSignerProviderAbility = "import-private-key" | "add-random-private-key";
interface IQedTransactionSigner {
    getPublicKeyHex(): Promise<string>;
    getPrivateKeyHex?(): Promise<string>;
    // signHash?(hash: QHashOut): Promise<ProofWithPublicInputs>;
    signAndSubmit(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getAbilities(): TQedTransactionSignerAbility[];
    registerUser(privateKeyHex: string): Promise<string>;
    addUser(privateKeyHex: string): Promise<string>;
    getClaimRewardsCallArgs(pk_hash: string, checkpointId: bigint, jobInfos: JobInfo[]): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string,  checkpointId: bigint, jobInfos: JobInfo[]): Promise<string>;
}

interface IQedTransactionSignerProvider {
    getSigners(): Promise<IQedTransactionSigner[]>;
    getPublicKeysHex(): Promise<string[]>;
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IQedTransactionSigner>;
    getAbilities(): TQedTransactionSignerProviderAbility[];
    importPrivateKey?(privateKeyHex: string): Promise<IQedTransactionSigner>;
    addRandomPrivateKey?(): Promise<IQedTransactionSigner>;
    registerUser(privateKeyHex: string): Promise<string>;
    addUser(privateKeyHex: string): Promise<string>;
    getClaimRewardsCallArgs(pk_hash: string, checkpointId: bigint, jobInfos: JobInfo[]): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string,  checkpointId: bigint, jobInfos: JobInfo[]): Promise<string>;
}

export type {
    IQedTransactionSigner,
    TQedTransactionSignerAbility,
    IQedTransactionSignerProvider,
    TQedTransactionSignerProviderAbility,
};
