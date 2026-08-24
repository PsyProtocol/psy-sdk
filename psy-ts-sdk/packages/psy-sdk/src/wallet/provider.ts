import { Felt } from "../core";
import { getPsyNetworkMagicForNetworkId, NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider, MultiCoordinatorRpcProvider } from "../coord-edge-rpc";
import { ContractCallData, IPsyUserProverProvider, ViewCallData } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider, MultiRealmRpcProvider } from "../realm-edge-rpc";
import { IPsyTransactionSignerProvider, PsyMemoryTransactionSignerProvider } from "../zksigner";
import { IPsyUserWallet, IPsyUserWalletProvider } from "./types";
import { PsyUserWallet } from "./userWallet";
import { PsyNetworkConfig } from "../config";
import { PsyWasmWebProverProvider } from "../local-web-prover";

/**
 * Minimal read-only interface for contract state access.
 * Read-only consumers (explorer, dashboards) can implement just this.
 */
export interface IContractStateReader {
    getContractState(checkpointId: Felt, contractId: Felt, userId: Felt, slots: Felt[]): Promise<Felt[]>;
    getLatestCheckpointId?(): Promise<Felt>;
}

/**
 * Full provider: read, write, and wallet-identity-bound view execution.
 */
export interface IContractProvider extends IContractStateReader {
    sendTransaction(
        contractId: Felt,
        functionName: string,
        args: Array<Felt | string>,
        publicKey: string
    ): Promise<unknown>;

    /** Execute a view (read-only) contract function for an explicit wallet identity. */
    callViewFunction(
        contractId: Felt,
        functionName: string,
        args: Array<Felt | string>,
        publicKey: string,
    ): Promise<Felt[]>;
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
        if (!Array.isArray(slots)) {
            throw new Error(`slots must be an array, got ${typeof slots}`);
        }
        const slotValues = await this.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getSlotValues(checkpointId, userId, contractId, slots);
        return slotValues;
    }

    async sendTransaction(
        contractId: Felt,
        functionName: string,
        args: Array<Felt | string>,
        publicKey: string
    ): Promise<unknown> {
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

    async callViewFunction(
        contractId: Felt,
        functionName: string,
        args: Array<Felt | string>,
        publicKey: string,
    ): Promise<Felt[]> {
        const normalizedPublicKey = publicKey.trim();
        if (!normalizedPublicKey) {
            throw new Error(`Cannot call view function "${functionName}": a wallet public key is required`);
        }
        await this.signerProvider.getSignerByPublicKeyHex(normalizedPublicKey);
        const callData: ViewCallData = {
            contract_calls: [
                {
                    contract_id: BigInt(contractId),
                    method_name: functionName,
                    inputs: args.map((arg) => BigInt(arg)),
                },
            ],
        };
        const response = await this.prover.callView(normalizedPublicKey, callData);
        const contractCall = response.contract_calls[0];
        if (!contractCall) {
            throw new Error(`View function "${functionName}" returned no contract call result`);
        }
        return contractCall.outputs;
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

    const userProver: IPsyUserProverProvider = new PsyWasmWebProverProvider(config);
    console.log("User Prover:", userProver);

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
