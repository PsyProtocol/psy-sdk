import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { IPsyUserProverProvider, PrepareRewardClaimRequest, PreparedRewardClaim, RewardClaimConfirmation, RewardClaimPreparation, SignType } from "../../local-prover-rpc";
import { cryptoRandomHashOutHex } from "../../utils";
import { IPsyTransactionSigner, IPsyTransactionSignerProvider, TPsyTransactionSignerProviderAbility } from "../types";
import { PsyMemoryTransactionSigner } from "./signer";
import { signatureFingerprints } from "./signature-fingerprints";

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
        if ([SignType.ZKSign, SignType.SECP256K1Sign, SignType.EthPersonalSECP256K1Sign].includes(signType)
            && fingerprint !== this.getFingerprintForSignType(signType)) {
            throw new Error("Built-in signature fingerprint mismatch");
        }
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
                return signatureFingerprints.zk;
            case SignType.SECP256K1Sign:
                return signatureFingerprints.secp256k1;
            case SignType.EthPersonalSECP256K1Sign:
                return signatureFingerprints["eth-personal-secp256k1"];
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

    async prepareRewardClaim(pkHash: string, request: PrepareRewardClaimRequest): Promise<RewardClaimPreparation> {
        return this.proverProvider.prepareRewardClaim(pkHash, request);
    }

    async confirmRewardClaim(l1RpcUrl: string, prepared: PreparedRewardClaim, txHash: string): Promise<RewardClaimConfirmation> {
        return this.proverProvider.confirmRewardClaim(l1RpcUrl, prepared, txHash);
    }
}

export { PsyMemoryTransactionSignerProvider };
