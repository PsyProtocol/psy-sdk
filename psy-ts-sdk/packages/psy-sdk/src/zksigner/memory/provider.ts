import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { IPsyUserProverProvider, PrepareRewardClaimRequest, PreparedRewardClaim, RewardClaimConfirmation, RewardClaimPreparation, SignType } from "../../local-prover-rpc";
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
        // QHashOut string form of the measured fingerprints in
        // psy_prover::wallet::memory_wallet (limbs reversed, each as 16-hex
        // big-endian), for stage magic 0x1337CF514544CF69 (localhost/testnet).
        switch (signType) {
            case SignType.ZKSign:
                return "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";
            case SignType.SECP256K1Sign:
                return "b3fe2b5b448751fa150f312ed92e9e2d7ab14c1eff9d43dcab36b57ad44f06a3";
            case SignType.EthPersonalSECP256K1Sign:
                // Canonical get_eth_personal_secp256k1_fingerprint() — the Rust
                // adapter resolves this internally; the TS held-key path mirrors
                // the existing hardcoded-fingerprint convention for local identity.
                // Measured against the rebuilt local-web-prover WASM: the
                // add_user dispatch accepts exactly this value and rejects the
                // pre-repin fingerprints.
                return "ecebdc4f04de4300441dd9fa0704c905c7d2c4db90ceb612459a266481b88a6d";
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
