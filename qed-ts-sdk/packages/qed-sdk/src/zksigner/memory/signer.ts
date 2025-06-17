import { getQedNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, DPNFunctionCircuitDefinition, IQEDUserProverProvider } from "../../local-prover-rpc";
import { IQedTransactionSigner, TQedTransactionSignerAbility } from "../types";

class QedMemoryTransactionSigner implements IQedTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    prover: IQEDUserProverProvider;
    private constructor(proverProvider: IQEDUserProverProvider, networkId: NetworkId, publicKeyHex: string, privateKeyHex: string) {
        this.networkId = networkId;
        this.networkMagic = getQedNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
    }
    static async create(proverProvider: IQEDUserProverProvider, networkId: NetworkId, privateKeyHex: string) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex);
        return new QedMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }
    // async signHash(hash: QHashOut): Promise<ProofWithPublicInputs> {
    //     return this.prover.getZKSignature(hash);
    // }

    async signAndSubmit(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string> {
        await this.prover.startSession(pk_hash);
        if (contractCallArgs instanceof Array) {
            await this.prover.proveContractCalls(pk_hash, contractCallArgs);
        } else {
            await this.prover.proveContractCall(pk_hash, contractCallArgs);
        }
        return this.prover.signAndSubmit(pk_hash);
    }

    async deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.prover.deployContract(pk_hash, circuitDefs);
    }

    getAbilities(): TQedTransactionSignerAbility[] {
        return ["sign-hash", "export-private-key-hex"];
    }

    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }
}

export { QedMemoryTransactionSigner };
