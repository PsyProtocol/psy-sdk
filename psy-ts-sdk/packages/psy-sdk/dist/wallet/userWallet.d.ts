import { IPsyUserWallet, IPsyCompleteUserInfo } from "./types";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { ClaimBatchItem, ContractCallArgs, DPNFunctionCircuitDefinition, GeneratedTxTraceJson, ProveTxTraceResumableJson, TraceProofConcurrentResult, TxMetadata } from "../local-prover-rpc";
import { PsyUserLeaf } from "../types";
import { IPsyTransactionSigner } from "../zksigner";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { NetworkId } from "../action";
declare class PsyUserWallet implements IPsyUserWallet {
    networkId: NetworkId;
    networkMagic: bigint;
    coordinator: ICoordinatorEdgeRpcProvider;
    realm: IRealmEdgeRpcProvider;
    signer: IPsyTransactionSigner;
    userId: number;
    publicKeyHex: string;
    status: boolean;
    constructor(networkId: NetworkId, signer: IPsyTransactionSigner, coordinator: ICoordinatorEdgeRpcProvider, realm: IRealmEdgeRpcProvider, userId: number, publicKeyHex: string, status: boolean);
    refresh(): Promise<PsyUserLeaf>;
    getUserInfo(): Promise<IPsyCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
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
export { PsyUserWallet };
//# sourceMappingURL=userWallet.d.ts.map