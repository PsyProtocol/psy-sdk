import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
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
    async importPrivateKey(privateKeyHex: string, signType: SignType, fingerprint: string): Promise<IPsyTransactionSigner> {
        const existing = this.signers.find((signer) => signer.privateKeyHex === privateKeyHex && signer.signType === signType && signer.fingerprint == fingerprint);
        if (existing) return existing;
        const signer = await PsyMemoryTransactionSigner.create(this.proverProvider, this.networkId, privateKeyHex, signType, fingerprint);
        this.signers.push(signer);
        return signer;
    }
    async addRandomPrivateKey(signType: SignType): Promise<IPsyTransactionSigner> {
        const fingerprint = this.getFingerprintForSignType(signType);
        return this.importPrivateKey(cryptoRandomHashOutHex(), signType, fingerprint);
    }

    private getFingerprintForSignType(signType: SignType): string {
        switch (signType) {
            case SignType.ZKSign:
                return "d2f572f1402fa8a92c9af0a2226e05ef8f5f4f34d764c6515b90d2b391fc48c1";
            case SignType.SECP256K1Sign:
                return "993bbdad2ba78319a70ab7d9ecd84b36eca0affc9f8ec4f9006b39a8fe29672c";
            default:
                throw new Error(`Unsupported sign type: ${signType}`);
        }
    }

    async registerUser(privateKeyHex: string, signType: SignType): Promise<string> {
        return this.proverProvider.registerUser(privateKeyHex, signType);
    }

    async addUser(privateKeyHex: string, signType: SignType): Promise<string> {
        return this.proverProvider.addUser(privateKeyHex, signType);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        return this.proverProvider.getClaimRewardsCallArgs(jobInfos);
    }

    async claimRewards(pk_hash: string, jobInfos: string): Promise<string> {
        return this.proverProvider.claimRewards(pk_hash, jobInfos);
    }
}

export { PsyMemoryTransactionSignerProvider };
