import { userWalletCache } from "./cache";
import { IPsyUserWallet, IPsyCompleteUserInfo } from "./types";
import { ICoordinatorEdgeRpcProvider } from "../coord-edge-rpc";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    TxMetadata,
} from "../local-prover-rpc";
import { PsyUserLeaf } from "../types";
import { psyFelt } from "../utils";
import { IPsyTransactionSigner } from "../zksigner";
import { IRealmEdgeRpcProvider } from "../realm-edge-rpc";
import { getPsyNetworkMagicForNetworkId, NetworkId } from "../action";

class PsyUserWallet implements IPsyUserWallet {
    networkId: NetworkId;
    networkMagic: bigint;
    coordinator: ICoordinatorEdgeRpcProvider;
    realm: IRealmEdgeRpcProvider;
    signer: IPsyTransactionSigner;

    userId: number;
    publicKeyHex: string;
    status: boolean;

    constructor(
        networkId: NetworkId,
        signer: IPsyTransactionSigner,
        coordinator: ICoordinatorEdgeRpcProvider,
        realm: IRealmEdgeRpcProvider,
        userId: number,
        publicKeyHex: string,
        status: boolean,
    ) {
        this.networkId = networkId;
        this.networkMagic = getPsyNetworkMagicForNetworkId(this.networkId);
        this.signer = signer;
        this.coordinator = coordinator;
        this.realm = realm;

        this.userId = userId;
        this.publicKeyHex = publicKeyHex;
        this.status = status;
    }

    async refresh(): Promise<PsyUserLeaf> {
        const publicKeyHex = await this.signer.getPublicKeyHex();
        try {
            const userId = await this.coordinator.getUserId(publicKeyHex);
            const { user, cache } = await userWalletCache.refreshUserFull(this.realm, userId);

            user.balance = cache.localBalance;
            user.nonce = cache.localNonce;
            this.status = true;
            return user;
        } catch (e) {
            console.warn("Error refreshing user wallet:", e);
            this.status = false;
            return {
                public_key: this.publicKeyHex,
                user_state_tree_root: this.publicKeyHex,
                balance: BigInt(0),
                nonce: BigInt(0),
                last_checkpoint_id: BigInt(0),
                event_index: BigInt(0),
                user_id: BigInt(0),
            };
        }
    }

    async getUserInfo(): Promise<IPsyCompleteUserInfo> {
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
        return psyFelt(b.balance);
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
    //     this.singer = new PsyMemoryTransactionSigner(this.prover, pkHash, sk);

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
        const callData = {
            contract_calls: Array.isArray(contractCallArgs) ? contractCallArgs : [contractCallArgs],
            software_defined_call: { "inputs": [] }
        };
        return this.signer.signAndSubmit(pk_hash, callData);
    }

    async execContractCallWithTrace(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<TxMetadata> {
        const callData = {
            contract_calls: Array.isArray(contractCallArgs) ? contractCallArgs : [contractCallArgs],
            software_defined_call: { "inputs": [] }
        };
        return this.signer.execContractCallWithTrace(pk_hash, callData);
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

export { PsyUserWallet };
