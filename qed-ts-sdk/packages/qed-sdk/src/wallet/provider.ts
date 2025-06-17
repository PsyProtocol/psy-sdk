import { getQedNetworkMagicForNetworkId, NetworkId } from "../action";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { IQEDUserProverProvider } from "../local-prover-rpc";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { IQedTransactionSignerProvider } from "../zksigner";
import { IQedUserWallet, IQedUserWalletProvider } from "./types";
import { QedUserWallet } from "./userWallet";


class QEdUserWalletProvider implements IQedUserWalletProvider {
  networkId: NetworkId;
  l2NetworkMagic: bigint;
  signerProvider: IQedTransactionSignerProvider;
  // rpc: IQedRPCProvider;
  coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider;
  realmEdgeRpcProvider: IRealmEdgeRpcProvider;
  prover: IQEDUserProverProvider;
  constructor(networkId: NetworkId, coordinatorEdgeRpcProvider: ICoordinatorEdgeRpcProvider, realmEdgeRpcProvider: IRealmEdgeRpcProvider, signerProvider: IQedTransactionSignerProvider, prover: IQEDUserProverProvider) {
    this.networkId = networkId;
    this.coordinatorEdgeRpcProvider = coordinatorEdgeRpcProvider;
    this.realmEdgeRpcProvider = realmEdgeRpcProvider;
    this.l2NetworkMagic = getQedNetworkMagicForNetworkId(networkId);
    this.signerProvider = signerProvider;
    this.prover = prover;
  }
  async getUserWallets(): Promise<IQedUserWallet[]> {
    const signers = await this.signerProvider.getSigners();
    const publicKeys = await Promise.all(signers.map(signer => signer.getPublicKeyHex()));
    const userIds = await Promise.all(publicKeys.map(publicKey => this.coordinatorEdgeRpcProvider.getUserId(publicKey)));
    return userIds.map((uid, index) => new QedUserWallet(this.networkId, signers[index], this.coordinatorEdgeRpcProvider, this.realmEdgeRpcProvider, uid, publicKeys[index]));
  }
}

export {
  QEdUserWalletProvider,
}