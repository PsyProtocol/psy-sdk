import { getQedNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, IQedUserProverProvider } from "../../local-prover-rpc";
import { JobInfo } from "../../types";
import { cryptoRandomHashOutHex } from "../../utils";
import { IQedTransactionSigner, IQedTransactionSignerProvider, TQedTransactionSignerProviderAbility } from "../types";
import { QedMemoryTransactionSigner } from "./signer";

class QedMemoryTransactionSignerProvider implements IQedTransactionSignerProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signers: QedMemoryTransactionSigner[] = [];
    proverProvider: IQedUserProverProvider;
    constructor(proverProvider: IQedUserProverProvider, networkId: NetworkId) {
        this.networkId = networkId;
        this.l2NetworkMagic = getQedNetworkMagicForNetworkId(networkId);
        this.proverProvider = proverProvider;
    }
    getSigners(): Promise<IQedTransactionSigner[]> {
        return Promise.resolve(this.signers);
    }
    getPublicKeysHex(): Promise<string[]> {
        return Promise.resolve(this.signers.map((signer) => signer.publicKeyHex));
    }
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IQedTransactionSigner> {
        const signer = this.signers.find((signer) => signer.publicKeyHex === publicKeyHex);
        if (!signer) return Promise.reject(new Error("Signer not found"));
        return Promise.resolve(signer);
    }
    getAbilities(): TQedTransactionSignerProviderAbility[] {
        return ["import-private-key", "add-random-private-key"];
    }
    async importPrivateKey(privateKeyHex: string): Promise<IQedTransactionSigner> {
        const existing = this.signers.find((signer) => signer.privateKeyHex === privateKeyHex);
        if (existing) return existing;
        const signer = await QedMemoryTransactionSigner.create(this.proverProvider, this.networkId, privateKeyHex);
        this.signers.push(signer);
        return signer;
    }
    addRandomPrivateKey(): Promise<IQedTransactionSigner> {
        return this.importPrivateKey(cryptoRandomHashOutHex());
    }

    async registerUser(privateKeyHex: string): Promise<string> {
        return this.proverProvider.registerUser(privateKeyHex);
    }

    async addUser(privateKeyHex: string): Promise<string> {
        return this.proverProvider.addUser(privateKeyHex);
    }

    async getClaimRewardsCallArgs(pk_hash: string, checkpointId: bigint, jobInfos: JobInfo[]): Promise<ContractCallArgs[]> {
        return this.proverProvider.getClaimRewardsCallArgs(pk_hash, checkpointId, jobInfos);
    }

    async claimRewards(pk_hash: string,  checkpointId: bigint, jobInfos: JobInfo[]): Promise<string> {
        return this.proverProvider.claimRewards(pk_hash, checkpointId, jobInfos);
    }
}

export { QedMemoryTransactionSignerProvider };
