import { userWalletCache } from "./cache";
import { IQedUserWallet, IQedCompleteUserInfo } from "./types";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import { PrivateKey, PublicKey } from "../core";
import {
    IQEDUserProverProvider,
    WalletKeyPair,
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
} from "../local-prover-rpc";
import { QEDUserLeaf } from "../types";
import { qedFelt } from "../utils";
import { IQedTransactionSigner } from "../zksigner";

class QedUserWallet implements IQedUserWallet {
    coordinator: ICoordinatorEdgeRpcProvider;
    prover: IQEDUserProverProvider;
    singer: IQedTransactionSigner;

    constructor(
        prover: IQEDUserProverProvider,
        coordinator: ICoordinatorEdgeRpcProvider,
        singer: IQedTransactionSigner
    ) {
        this.prover = prover;
        this.coordinator = coordinator;
        this.singer = singer;
    }

    async refresh(): Promise<QEDUserLeaf> {
        const publicKeyHex = await this.singer.getPublicKeyHex();
        const userId = await this.coordinator.getUserId(publicKeyHex);
        const { user, cache } = await userWalletCache.refreshUserFull(this.coordinator, userId);

        user.balance = cache.localBalance;
        user.nonce = cache.localNonce;

        return user;
    }

    async getUserInfo(): Promise<IQedCompleteUserInfo> {
        const user = await this.refresh();
        const publicKeyHex = await this.singer.getPublicKeyHex();
        return Promise.resolve({
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

    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.prover.registerUser(privateKey);
    }

    async getZKPublicKey(): Promise<PublicKey> {
        return this.singer.getPublicKeyHex();
    }

    async importPrivateKey(privateKey: PrivateKey): Promise<PublicKey> {
        return this.prover.addUser(privateKey);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.prover.getRandomKeypair();
    }

    async deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.singer.signAndSubmit(() => this.prover.deployContract(circuitDefs));
    }

    async contractCall(contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.singer.signAndSubmit(() => this.prover.proveContractCalls(contractCallArgs));
    }

    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    async transfer(_recipient: string, _amount: string, _nonce?: string): Promise<void> {
        // const contractCallArgs: ContractCallArgs = {
        //     contract_id: BigInt(0),
        //     method_name: "transfer",
        //     inputs: [BigInt(recipient), BigInt(amount), BigInt(nonce)],
        // };
        // await this.contractCall([contractCallArgs]);
    }
}

export { QedUserWallet };
