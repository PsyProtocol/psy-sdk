import { ContractCallArgs, IQEDUserProverProvider, ProofWithPublicInputs } from "../../local-prover-rpc";
import { QHashOut } from "../../types";
import { IQedTransactionSigner, TQedTransactionSignerAbility } from "../types";

class QedMemoryTransactionSigner implements IQedTransactionSigner {
    publicKeyHex: string;
    privateKeyHex: string;
    prover: IQEDUserProverProvider;
    private constructor(proverProvider: IQEDUserProverProvider, publicKeyHex: string, privateKeyHex: string) {
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
    }
    static async create(proverProvider: IQEDUserProverProvider, privateKeyHex: string) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex);
        return new QedMemoryTransactionSigner(proverProvider, publicKeyHex, privateKeyHex);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }
    async signHash(hash: QHashOut): Promise<ProofWithPublicInputs> {
        return this.prover.getZKSignature(hash);
    }

    async signAndSubmit(contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string> {
        await this.prover.switchUser(this.publicKeyHex);
        await this.prover.startSession();
        if (contractCallArgs instanceof Array) {
            await this.prover.proveContractCalls(contractCallArgs);
        } else {
            await this.prover.proveContractCall(contractCallArgs);
        }
        return this.prover.signAndSubmit();
    }

    getAbilities(): TQedTransactionSignerAbility[] {
        return ["sign-hash", "export-private-key-hex"];
    }

    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }
}

export { QedMemoryTransactionSigner };
