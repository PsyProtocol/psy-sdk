import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, IPsyUserProverProvider } from "../../local-prover-rpc";
import { JobInfo } from "../../types";
import { cryptoRandomHashOutHex } from "../../utils";
import { IPsyTransactionSigner, IPsyTransactionSignerProvider, TPsyTransactionSignerProviderAbility } from "../types";
import { PsyMemoryTransactionSigner } from "./signer";

class PsyMemoryTransactionSignerProvider implements IPsyTransactionSignerProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signers: PsyMemoryTransactionSigner[] = [];
    proverProvider: IPsyUserProverProvider;
    constructor(proverProvider: IPsyUserProverProvider, networkId: NetworkId) {
        this.networkId = networkId;
        this.l2NetworkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.proverProvider = proverProvider;
    }
    getSigners(): Promise<IPsyTransactionSigner[]> {
        return Promise.resolve(this.signers);
    }
    getPublicKeysHex(): Promise<string[]> {
        return Promise.resolve(this.signers.map((signer) => signer.publicKeyHex));
    }
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IPsyTransactionSigner> {
        const signer = this.signers.find((signer) => signer.publicKeyHex === publicKeyHex);
        if (!signer) return Promise.reject(new Error("Signer not found"));
        return Promise.resolve(signer);
    }
    getAbilities(): TPsyTransactionSignerProviderAbility[] {
        return ["import-private-key", "add-random-private-key"];
    }
    async importPrivateKey(privateKeyHex: string, signType: string, fingerprint?: string): Promise<IPsyTransactionSigner> {
        const existing = this.signers.find((signer) => signer.privateKeyHex === privateKeyHex && signer.signType === signType && signer.fingerprint == fingerprint);
        if (existing) return existing;
        const signer = await PsyMemoryTransactionSigner.create(this.proverProvider, this.networkId, privateKeyHex, signType, fingerprint);
        this.signers.push(signer);
        return signer;
    }
    async addRandomPrivateKey(signType: string, fingerprint?: string): Promise<IPsyTransactionSigner> {
        return this.importPrivateKey(cryptoRandomHashOutHex(), signType, fingerprint);
    }

    async registerUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string> {
        return this.proverProvider.registerUserWithType(privateKeyHex, signType, fingerprint);
    }

    async addUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string> {
        return this.proverProvider.addUserWithType(privateKeyHex, signType, fingerprint);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        return this.proverProvider.getClaimRewardsCallArgs(jobInfos);
    }

    async claimRewards(pk_hash: string, jobInfos: string): Promise<string> {
        return this.proverProvider.claimRewards(pk_hash, jobInfos);
    }
}

export { PsyMemoryTransactionSignerProvider };
