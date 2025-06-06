import { IQEDUserProverProvider, ProofWithPublicInputs } from "../../local-prover-rpc";
import { QHashOut } from "../../types";
import { IQedTransactionSigner, TQedTransactionSignerAbility } from "../types";

class QedMemoryTransactionSigner implements IQedTransactionSigner {
    publicKeyHex: string;
    privateKeyHex: string;
    proverProvider: IQEDUserProverProvider;
    private constructor(proverProvider: IQEDUserProverProvider, publicKeyHex: string, privateKeyHex: string) {
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.proverProvider = proverProvider;
    }
    static async create(proverProvider: IQEDUserProverProvider, privateKeyHex: string) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex);
        return new QedMemoryTransactionSigner(proverProvider, publicKeyHex, privateKeyHex);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }
    async signHash(hash: QHashOut): Promise<ProofWithPublicInputs> {
        return this.proverProvider.getZKSignature(hash);
    }

    async signAndSubmit(callback: () => Promise<string>): Promise<string> {
        await this.proverProvider.switchUser(this.publicKeyHex);
        await this.proverProvider.startSession();
        await callback();
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
