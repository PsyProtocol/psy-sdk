import { getQedNetworkMagicForNetworkId, NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { IQedUserProverProvider } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { IQedTransactionSignerProvider } from "../zksigner";
import { IQedUserWallet, IQedUserWalletProvider } from "./types";
import { QedUserWallet } from "./userWallet";

class QedUserWalletProvider implements IQedUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IQedTransactionSignerProvider;
    // rpc: IQedRPCProvider;
    coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider;
    realmEdgeRpcProvider: IRealmEdgeRpcProvider;
    prover: IQedUserProverProvider;
    constructor(
        networkId: NetworkId,
        coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider,
        realmEdgeRpcProvider: IRealmEdgeRpcProvider,
        signerProvider: IQedTransactionSignerProvider,
        prover: IQedUserProverProvider
    ) {
        this.networkId = networkId;
        this.coordinatorEdgeRpcProvider = coordinatorEdgeRpcProvider;
        this.realmEdgeRpcProvider = realmEdgeRpcProvider;
        this.l2NetworkMagic = getQedNetworkMagicForNetworkId(networkId);
        this.signerProvider = signerProvider;
        this.prover = prover;
    }
    async getUserWallets(): Promise<IQedUserWallet[]> {
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
                new QedUserWallet(
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

export { QedUserWalletProvider };
