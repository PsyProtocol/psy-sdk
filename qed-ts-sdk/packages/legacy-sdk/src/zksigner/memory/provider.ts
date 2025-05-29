import { DogeNetworkId } from "doge-sdk";
import { CityMemoryTransactionSigner } from "./signer";
import { getCityNetworkMagicForNetworkId } from "../../action/constants";
import { ICityUserProverProvider } from "../../userProverRPC/types";
import { cryptoRandomHashOutHex } from "../../utils/felt";
import {
    ICityTransactionSigner,
    ICityTransactionSignerProvider,
    TCityTransactionSignerProviderAbility,
} from "../types";

class CityMemoryTransactionSignerProvider implements ICityTransactionSignerProvider {
    networkId: DogeNetworkId;
    l2NetworkMagic: string;
    signers: CityMemoryTransactionSigner[] = [];
    proverProvider: ICityUserProverProvider;
    constructor(proverProvider: ICityUserProverProvider, networkId: DogeNetworkId) {
        this.networkId = networkId;
        this.l2NetworkMagic = getCityNetworkMagicForNetworkId(networkId);
        this.proverProvider = proverProvider;
    }
    getSigners(): Promise<ICityTransactionSigner[]> {
        return Promise.resolve(this.signers);
    }
    getPublicKeysHex(): Promise<string[]> {
        return Promise.resolve(this.signers.map((signer) => signer.publicKeyHex));
    }
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<ICityTransactionSigner> {
        const signer = this.signers.find((signer) => signer.publicKeyHex === publicKeyHex);
        if (!signer) return Promise.reject(new Error("Signer not found"));
        return Promise.resolve(signer);
    }
    getAbilities(): TCityTransactionSignerProviderAbility[] {
        return ["import-private-key", "add-random-private-key"];
    }
    async importPrivateKey(privateKeyHex: string): Promise<ICityTransactionSigner> {
        const existing = this.signers.find((signer) => signer.privateKeyHex === privateKeyHex);
        if (existing) return existing;
        const signer = await CityMemoryTransactionSigner.create(this.proverProvider, this.networkId, privateKeyHex);
        this.signers.push(signer);
        return signer;
    }
    addRandomPrivateKey(): Promise<ICityTransactionSigner> {
        return this.importPrivateKey(cryptoRandomHashOutHex());
    }
}

export { CityMemoryTransactionSignerProvider };
