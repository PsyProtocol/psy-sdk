import { Felt } from "../core";
import { NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { IPsyUserProverProvider } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { IPsyTransactionSignerProvider } from "../zksigner";
import { IPsyUserWallet, IPsyUserWalletProvider } from "./types";
import { PsyNetworkConfig } from "../config";
export interface IContractProvider {
    getContractState(checkpointId: Felt, contractId: Felt, userId: Felt, slots: Felt[]): Promise<Felt[]>;
    sendTransaction(contractId: Felt, functionName: string, args: any[], publicKey: string): Promise<any>;
    getLatestCheckpointId?(): Promise<Felt>;
}
declare class PsyUserWalletProvider implements IPsyUserWalletProvider, IContractProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider;
    realmEdgeRpcProvider: IRealmEdgeRpcProvider;
    prover: IPsyUserProverProvider;
    constructor(networkId: NetworkId, coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider, realmEdgeRpcProvider: IRealmEdgeRpcProvider, signerProvider: IPsyTransactionSignerProvider, prover: IPsyUserProverProvider);
    getUserWallets(): Promise<IPsyUserWallet[]>;
    getContractState(checkpointId: Felt, contractId: Felt, userId: Felt, slots: Felt[]): Promise<Felt[]>;
    sendTransaction(contractId: Felt, functionName: string, args: any[], publicKey: string): Promise<any>;
    getLatestCheckpointId(): Promise<Felt>;
}
declare function createMemoryWalletProvider(config: PsyNetworkConfig): Promise<PsyUserWalletProvider>;
export { PsyUserWalletProvider, createMemoryWalletProvider };
//# sourceMappingURL=provider.d.ts.map