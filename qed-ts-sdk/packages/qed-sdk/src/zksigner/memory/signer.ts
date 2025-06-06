import { IQEDUserProverProvider, ProofWithPublicInputs } from "../../local-prover-rpc";
import { QHashOut } from "../../types";
import { IQedTransactionSigner, TQedTransactionSignerAbility } from "../types";

class QedMemoryTransactionSigner implements IQedTransactionSigner {
    publicKeyHex: string;
    privateKeyHex: string;
    proverProvider: IQEDUserProverProvider;
    constructor(proverProvider: IQEDUserProverProvider, publicKeyHex: string, privateKeyHex: string) {
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.proverProvider = proverProvider;
    }
    static async create(proverProvider: IQEDUserProverProvider, privateKeyHex: string) {
        const zkPublicKeyInfo = await proverProvider.getZKPublicKey(privateKeyHex);
        return new QedMemoryTransactionSigner(proverProvider, zkPublicKeyInfo.public_key_param, privateKeyHex);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }
    async signHash(hash: QHashOut): Promise<ProofWithPublicInputs> {
        return this.proverProvider.getZKSignature(hash);
    }

    async signAndSubmit(): Promise<string> {
        return this.proverProvider.signAndSubmit();
    }

    getAbilities(): TQedTransactionSignerAbility[] {
        return ["sign-hash", "export-private-key-hex"];
    }

    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }
}

export { QedMemoryTransactionSigner };
