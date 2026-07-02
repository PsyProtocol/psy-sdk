import { Felt } from "../core";
import {
    ClaimBatchItem,
    ContractCallArgs,
    GeneratedTxTraceJson,
    ProveTxTraceResumableJson,
    TraceProofConcurrentResult,
    TxMetadata,
} from "../local-prover-rpc";
import { IPsyTransactionSigner, IPsyTransactionSignerProvider } from "../zksigner";
import { NetworkId } from "../action";

interface ICorePsyUserInfo {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    userId: Felt;
    publicKeyHex: string;
}

interface IPsyCompleteUserInfo extends ICorePsyUserInfo {
    nonce: string;
    balance: Felt;
}

interface IPsyUserWallet {
    // prover: IPsyUserProverProvider;
    status: boolean;
    signer: IPsyTransactionSigner;
    getUserInfo(): Promise<IPsyCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    // getRandomKeypair(): Promise<WalletKeyPair>;
    // registerUser(privateKey: PrivateKey): Promise<PublicKey>;
    // getZKPublicKey(): Promise<PublicKey>;
    // importPrivateKey(privateKey: PrivateKey): Promise<PublicKey>;
    // deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    // getDeployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;
    execContractCall(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    execContractCallWithTrace(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<TxMetadata>;
    // Build a savable trace envelope (keyed by `sig_hash`) without proving/submitting, so the
    // wallet can persist it and prove/track it later via the step API.
    generateTxTrace(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<GeneratedTxTraceJson>;
    generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTraceJson>;
    proveTxTraceStep(pk_hash: string, envelope: string | GeneratedTxTraceJson, resumeFrom?: {
        proof_tree_meta: unknown;
        last_step_info: unknown;
        current_header: unknown;
        previous_header: unknown;
        proof_blobs: Uint8Array[];
        next_step_index: number;
    }): Promise<ProveTxTraceResumableJson>;
    proveTxTraceConcurrent(pk_hash: string, envelope: string | GeneratedTxTraceJson): Promise<TraceProofConcurrentResult>;
    // transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
}

interface IPsyUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    getUserWallets(): Promise<IPsyUserWallet[]>;
}

export type { ICorePsyUserInfo, IPsyUserWallet, IPsyCompleteUserInfo, IPsyUserWalletProvider };
