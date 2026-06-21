import { getPsyNetworkMagicForNetworkId } from '../../action/constants.mjs';
import '../../utils/address.mjs';
import { cryptoRandomHashOutHex } from '../../utils/felt.mjs';
import { SignType } from '../../local-prover-rpc/types.mjs';
import '../../utils/json.mjs';
import '../../utils/random.mjs';
import { PsyMemoryTransactionSigner } from './signer.mjs';

class PsyMemoryTransactionSignerProvider {
    constructor(proverProvider, networkId) {
        this.signers = [];
        this.networkId = networkId;
        this.l2NetworkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.proverProvider = proverProvider;
    }
    getSigners() {
        return Promise.resolve(this.signers);
    }
    getPublicKeysHex() {
        return Promise.resolve(this.signers.map((signer) => signer.publicKeyHex));
    }
    getSignerByPublicKeyHex(publicKeyHex) {
        const signer = this.signers.find((signer) => signer.publicKeyHex === publicKeyHex);
        if (!signer)
            return Promise.reject(new Error("Signer not found"));
        return Promise.resolve(signer);
    }
    getAbilities() {
        return ["import-private-key", "add-random-private-key"];
    }
    async importPrivateKey(privateKeyHex, signType, fingerprint) {
        const existing = this.signers.find((signer) => signer.privateKeyHex === privateKeyHex && signer.signType === signType && signer.fingerprint == fingerprint);
        if (existing)
            return existing;
        const signer = await PsyMemoryTransactionSigner.create(this.proverProvider, this.networkId, privateKeyHex, signType, fingerprint);
        this.signers.push(signer);
        return signer;
    }
    async addRandomPrivateKey(signType) {
        const fingerprint = this.getFingerprintForSignType(signType);
        return this.importPrivateKey(cryptoRandomHashOutHex(), signType, fingerprint);
    }
    getFingerprintForSignType(signType) {
        switch (signType) {
            case SignType.ZKSign:
                return "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";
            case SignType.SECP256K1Sign:
                return "320d034234f0dab4d02c4b03d69276cbd5c2eb831aca1b11c7e52078ace2e33b";
            default:
                throw new Error(`Unsupported sign type: ${signType}`);
        }
    }
    async registerUser(privateKeyHex, signType, fingerprint) {
        return this.proverProvider.registerUser(privateKeyHex, signType, fingerprint);
    }
    async addUser(privateKeyHex, signType, fingerprint) {
        return this.proverProvider.addUser(privateKeyHex, signType, fingerprint);
    }
    async getClaimRewardsCallArgs(jobInfos) {
        return this.proverProvider.getClaimRewardsCallArgs(jobInfos);
    }
    async claimRewards(pk_hash, jobInfos) {
        return this.proverProvider.claimRewards(pk_hash, jobInfos);
    }
}

export { PsyMemoryTransactionSignerProvider };
