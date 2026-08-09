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
                return "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";
            case SignType.SECP256K1Sign:
                return "320d034234f0dab4d02c4b03d69276cbd5c2eb831aca1b11c7e52078ace2e33b";
            case SignType.EthPersonalSECP256K1Sign:
                // Canonical get_eth_personal_secp256k1_fingerprint() — the Rust
                // adapter resolves this internally; the TS held-key path mirrors
                // the existing hardcoded-fingerprint convention for local identity.
                return "4cf514982eb7155648bf1b7852a6a564d8e86998cc1c6365a50e15796b7f0745";
            default:
                throw new Error(`Unsupported sign type: ${signType}`);
        }
    }

    async registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string> {
        return this.proverProvider.registerUser(privateKeyHex, signType, fingerprint);
    }

    async addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string> {
        return this.proverProvider.addUser(privateKeyHex, signType, fingerprint);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        return this.proverProvider.getClaimRewardsCallArgs(jobInfos);
    }

    async claimRewards(pk_hash: string, jobInfos: string): Promise<string> {
        return this.proverProvider.claimRewards(pk_hash, jobInfos);
    }
}

export { PsyMemoryTransactionSignerProvider };
