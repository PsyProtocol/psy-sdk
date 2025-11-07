import { getPsyNetworkMagicForNetworkId, NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { IPsyUserProverProvider } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { IPsyTransactionSignerProvider } from "../zksigner";
import { IPsyUserWallet, IPsyUserWalletProvider } from "./types";
import { PsyUserWallet } from "./userWallet";

class PsyUserWalletProvider implements IPsyUserWalletProvider {
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
}

export { PsyUserWalletProvider };
