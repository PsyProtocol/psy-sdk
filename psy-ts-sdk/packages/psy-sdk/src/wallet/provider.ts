import { Felt } from "../core";
import { getPsyNetworkMagicForNetworkId, NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider, MultiCoordinatorRpcProvider } from "../coord-edge-rpc";
import { ContractCallData, IPsyUserProverProvider } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider, MultiRealmRpcProvider } from "../realm-edge-rpc";
import { IPsyTransactionSignerProvider, PsyMemoryTransactionSignerProvider } from "../zksigner";
import { IPsyUserWallet, IPsyUserWalletProvider } from "./types";
import { PsyUserWallet } from "./userWallet";
import { PsyNetworkConfig } from "../config";
import { initWasmSync, PsyWasmWebProverProvider, WasmRpcServer } from "../local-web-prover";
import { PsyJSON } from "../utils";

export interface IContractProvider {
    getContractState(checkpointId: Felt, contractId: Felt, userId: Felt, slots: Felt[]): Promise<Felt[]>;

    sendTransaction(
        contractId: Felt,
        functionName: string,
        args: any[],
        publicKey: string
    ): Promise<any>;

    getLatestCheckpointId?(): Promise<Felt>;
}

class PsyUserWalletProvider implements IPsyUserWalletProvider, IContractProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    // rpc: IPsyRPCProvider;
    coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider;
    realmEdgeRpcProvider: IRealmEdgeRpcProvider;
    prover: IPsyUserProverProvider;
    constructor(
        networkId: NetworkId,
        coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider,
        realmEdgeRpcProvider: IRealmEdgeRpcProvider,
        signerProvider: IPsyTransactionSignerProvider,
        prover: IPsyUserProverProvider
    ) {
        this.networkId = networkId;
        this.coordinatorEdgeRpcProvider = coordinatorEdgeRpcProvider;
        this.realmEdgeRpcProvider = realmEdgeRpcProvider;
        this.l2NetworkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.signerProvider = signerProvider;
        this.prover = prover;
    }
    async getUserWallets(): Promise<IPsyUserWallet[]> {
        const signers = await this.signerProvider.getSigners();
        const publicKeys = await Promise.all(signers.map((signer) => signer.getPublicKeyHex()));
        const userIds = await Promise.all(
            publicKeys.map(async (publicKey) => {
                try {
                    return { userId: await this.coordinatorEdgeRpcProvider.getUserId(publicKey), status: true };
                } catch (error) {
                    console.warn(`Failed to get user ID for public key ${publicKey}:`, error);
                    return { userId: 0, status: false };
                }
            })
        );
        return userIds.map(
            ({ userId, status }, index) =>
                new PsyUserWallet(
                    this.networkId,
                    signers[index],
                    this.coordinatorEdgeRpcProvider,
                    this.realmEdgeRpcProvider.getRpcProviderByUserId(userId),
                    userId,
                    publicKeys[index],
                    status,
                )
        );
    }

    async getContractState(
        checkpointId: Felt,
        contractId: Felt,
        userId: Felt,
        slots: Felt[]
    ): Promise<Felt[]> {
        const userStateTreeHeight = (await this.coordinatorEdgeRpcProvider.getContractLeafData(contractId)).state_tree_height;
        if (!Array.isArray(slots)) {
            throw new Error(`slots must be an array, got ${typeof slots}`);
        }
        const slotValues = await this.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getSlotValues(checkpointId, userId, contractId, Number(userStateTreeHeight), slots);
        return slotValues;
    }

    async sendTransaction(
        contractId: Felt,
        functionName: string,
        args: any[],
        publicKey: string
    ): Promise<any> {
        const signer = await this.signerProvider.getSignerByPublicKeyHex(publicKey);
        const contractCallData: ContractCallData = {
            contract_calls: [
                {
                    contract_id: BigInt(contractId),
                    method_name: functionName,
                    inputs: args.map((arg) => BigInt(arg))
                }
            ],
            software_defined_call: {
                "inputs": []
            }
        };
        return signer.signAndSubmit(publicKey, contractCallData);
    }

    async getLatestCheckpointId(): Promise<Felt> {
        const latestState = await this.coordinatorEdgeRpcProvider.getLatestBlockState();
        return latestState.checkpoint_id;
    }
}

async function createMemoryWalletProvider(
    config: PsyNetworkConfig,
): Promise<PsyUserWalletProvider> {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(
        config.coordinator_configs,
    );
    const realm_rpc = new MultiRealmRpcProvider(
        config.realm_configs,
        config.users_per_realm,
    );

    const initWasmRpcServer = async () => {
        try {
            const json = PsyJSON.stringify(config);
            const now = new Date().getTime();
            initWasmSync();
            PsyWasmWebProverProvider.wasmServer = await new WasmRpcServer(json);
            console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
        } catch (error) {
            console.error('Failed to get prover URL:', error);
        }
    };
    await initWasmRpcServer();

    let userProver: IPsyUserProverProvider = new PsyWasmWebProverProvider(config);
    console.log("User Prover:", userProver);
    console.log("User Prover:", PsyWasmWebProverProvider.wasmServer);


    const transactionSignerProvider = new PsyMemoryTransactionSignerProvider(
        userProver,
        networkId,
    );

    return new PsyUserWalletProvider(
        networkId,
        coordinator_rpc,
        realm_rpc,
        transactionSignerProvider,
        userProver,
    );
}

export { PsyUserWalletProvider, createMemoryWalletProvider };
