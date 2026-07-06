import { Felt } from "../core";
import { ClaimBatchItem, ContractCallArgs, GeneratedTxTraceJson, ProveTxTraceResumableJson, TraceProofConcurrentResult, TxMetadata } from "../local-prover-rpc";
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
    status: boolean;
    signer: IPsyTransactionSigner;
    getUserInfo(): Promise<IPsyCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    execContractCall(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    execContractCallWithTrace(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<TxMetadata>;
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
}
interface IPsyUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    getUserWallets(): Promise<IPsyUserWallet[]>;
}
export type { ICorePsyUserInfo, IPsyUserWallet, IPsyCompleteUserInfo, IPsyUserWalletProvider };
//# sourceMappingURL=types.d.ts.map