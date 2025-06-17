import { userWalletCache } from "./cache";
import { IQedUserWallet, IQedCompleteUserInfo } from "./types";
import { CoordinatorEdgeRpcProvider, ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { PrivateKey, PublicKey } from "../core";
import {
    IQEDUserProverProvider,
    WalletKeyPair,
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    QBCDeployContract,
    QEDRPCUserProverProvider,
} from "../local-prover-rpc";
import { QEDUserLeaf } from "../types";
import { qedFelt } from "../utils";
import { IQedTransactionSigner, QedMemoryTransactionSigner } from "../zksigner";
import { IRealmEdgeRpcProvider, RealmEdgeRpcProvider } from "../realm-edge-rpc";
import { getQedNetworkMagicForNetworkId, NetworkId } from "../action";

class QedUserWallet implements IQedUserWallet {
    networkId: NetworkId;
    networkMagic: bigint;
    coordinator: ICoordinatorEdgeRpcProvider;
    realm: IRealmEdgeRpcProvider;
    // prover: IQEDUserProverProvider;
    signer: IQedTransactionSigner;

    userId: number;
    publicKeyHex: string;

    constructor(networkId: NetworkId, signer: IQedTransactionSigner, coordinator: ICoordinatorEdgeRpcProvider,
        realm: IRealmEdgeRpcProvider, userId: number, publicKeyHex: string) {
        this.networkId = networkId;
        this.networkMagic = getQedNetworkMagicForNetworkId(this.networkId);
        this.signer = signer;
        this.coordinator = coordinator;
        this.realm = realm;

        this.userId = userId;
        this.publicKeyHex = publicKeyHex;
    }

    async refresh(): Promise<QEDUserLeaf> {
        const publicKeyHex = await this.signer.getPublicKeyHex();
        const userId = await this.coordinator.getUserId(publicKeyHex);
        const { user, cache } = await userWalletCache.refreshUserFull(this.realm, userId);

        user.balance = cache.localBalance;
        user.nonce = cache.localNonce;

        return user;
    }

    async getUserInfo(): Promise<IQedCompleteUserInfo> {
        const user = await this.refresh();
        const publicKeyHex = await this.signer.getPublicKeyHex();
        return Promise.resolve({
            networkId: this.networkId,
            l2NetworkMagic: this.networkMagic,
            nonce: user.nonce.toString(10),
            balance: user.balance,
            userId: user.user_id,
            publicKeyHex: publicKeyHex,
        });
    }

    async getBalance(): Promise<bigint> {
        const b = await this.refresh();
        return qedFelt(b.balance);
    }

    async getBalanceString(): Promise<string> {
        const balance = await this.getBalance();
        return balance.toString();
    }

    // async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
    //     const pk = await this.prover.registerUser(privateKey);
    //     this.accounts.set(pk, privateKey)
    //     return pk
    // }

    // async addUser(privateKey: PrivateKey): Promise<PublicKey> {
    //     const pk = await this.prover.addUser(privateKey);
    //     this.accounts.set(pk, privateKey)
    //     return pk
    // }

    // async switchUser(pkHash: PublicKey): Promise<void> {
    //     await this.prover.switchUser(pkHash);
    //     const sk = this.accounts.get(pkHash);
    //     if (!sk) {
    //         throw new Error("private key not found");
    //     }
    //     this.singer = new QedMemoryTransactionSigner(this.prover, pkHash, sk);

    //     const userId = await this.coordinator.getUserId(pkHash);
    //     this.realm.setUserId(userId)
    // }

    // async getZKPublicKey(): Promise<PublicKey> {
    //     return this.signer.getPublicKeyHex();
    // }

    // async importPrivateKey(privateKey: PrivateKey): Promise<PublicKey> {
    //     return this.prover.addUser(privateKey);
    // }

    // async getRandomKeypair(): Promise<WalletKeyPair> {
    //     return this.prover.getRandomKeypair();
    // }

    async deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.signer.deployContract(pk_hash, circuitDefs);
    }

    // async getDeployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract> {
    //     // await this.prover.switchUser(await this.getZKPublicKey());
    //     // await this.prover.startSession();
    //     return this.prover.getDeployContractCmd(circuitDefs);
    // }

    async execContractCall(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string> {
        return this.signer.signAndSubmit(pk_hash, contractCallArgs);
    }

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    // async transfer(_recipient: string, _amount: string, _nonce?: string): Promise<void> {
    //     // const contractCallArgs: ContractCallArgs = {
    //     //     contract_id: BigInt(0),
    //     //     method_name: "transfer",
    //     //     inputs: [BigInt(recipient), BigInt(amount), BigInt(nonce)],
    //     // };
    //     // await this.contractCall([contractCallArgs]);
    // }

    // async proveSession(contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string> {
    //     // await this.prover.startSession();
    //     return this.signer.signAndSubmit(contractCallArgs);
    // }
}

export { QedUserWallet };
