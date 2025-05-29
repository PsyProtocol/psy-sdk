import { DogeNetworkId } from "doge-sdk/dist/types";
import { ICityUserWallet, ICityUserWalletProvider } from "./types";
import { CityUserWallet } from "./userWallet";
import { getCityNetworkMagicForNetworkId } from "../action/constants";
import { ICityRPCProvider } from "../rpc/types";
import { ICityUserProverProvider } from "../userProverRPC/types";
import { ICityTransactionSignerProvider } from "../zksigner/types";

class CityUserWalletProvider implements ICityUserWalletProvider {
    networkId: DogeNetworkId;
    l2NetworkMagic: string;
    signerProvider: ICityTransactionSignerProvider;
    rpc: ICityRPCProvider;
    prover: ICityUserProverProvider;
    constructor(
        networkId: DogeNetworkId,
        rpc: ICityRPCProvider,
        signerProvider: ICityTransactionSignerProvider,
        prover: ICityUserProverProvider
    ) {
        this.networkId = networkId;
        this.rpc = rpc;
        this.l2NetworkMagic = getCityNetworkMagicForNetworkId(networkId);
        this.signerProvider = signerProvider;
        this.prover = prover;
    }
    async getUserWallets(): Promise<ICityUserWallet[]> {
        const signers = await this.signerProvider.getSigners();
        const publicKeys = await Promise.all(signers.map((signer) => signer.getPublicKeyHex()));
        const userIds = await Promise.all(publicKeys.map((publicKey) => this.rpc.getUserIdsForPublicKey(publicKey)));
        return userIds
            .map((uids, index) =>
                uids.map((uid) => new CityUserWallet(signers[index], this.rpc, uid, publicKeys[index]))
            )
            .reduce((acc, wallets) => acc.concat(wallets), []);
    }
}

export { CityUserWalletProvider };
