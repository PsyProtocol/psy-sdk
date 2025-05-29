import { DogeNetworkId } from "doge-sdk";
import { getCityNetworkMagicForNetworkId } from "../../action/constants";
import { computeSigActionHash } from "../../action/sighash";
import { ICitySigAction } from "../../action/types";
import { ICityUserProverProvider } from "../../userProverRPC/types";
import { hashOutHex } from "../../utils/felt";
import { ICityTransactionSigner, TCityTransactionSignerAbility } from "../types";

class CityMemoryTransactionSigner implements ICityTransactionSigner {
    networkId: DogeNetworkId;
    l2NetworkMagic: string;
    publicKeyHex: string;
    privateKeyHex: string;
    proverProvider: ICityUserProverProvider;
    constructor(
        proverProvider: ICityUserProverProvider,
        networkId: DogeNetworkId,
        publicKeyHex: string,
        privateKeyHex: string
    ) {
        this.networkId = networkId;
        this.l2NetworkMagic = getCityNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.proverProvider = proverProvider;
    }
    static async create(proverProvider: ICityUserProverProvider, networkId: DogeNetworkId, privateKeyHex: string) {
        const publicKeyHex = await proverProvider.getZKPublicKey(privateKeyHex);
        return new CityMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }
    async signHash(hash: string): Promise<string> {
        return this.proverProvider.proveZKSignature(this.privateKeyHex, hash);
    }
    async signSigAction(sigAction: ICitySigAction): Promise<string> {
        return this.signHash(hashOutHex(computeSigActionHash(sigAction)));
    }
    getAbilities(): TCityTransactionSignerAbility[] {
        return ["sign-hash", "sign-sigaction", "export-private-key-hex"];
    }
    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }
}

export { CityMemoryTransactionSigner };
