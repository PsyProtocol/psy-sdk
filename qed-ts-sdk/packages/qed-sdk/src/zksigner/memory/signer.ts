import { getQedNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, DPNFunctionCircuitDefinition, IQedUserProverProvider } from "../../local-prover-rpc";
import { IQedTransactionSigner, TQedTransactionSignerAbility } from "../types";

class QedMemoryTransactionSigner implements IQedTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    prover: IQedUserProverProvider;
    private constructor(
        proverProvider: IQedUserProverProvider,
        networkId: NetworkId,
        publicKeyHex: string,
        privateKeyHex: string
    ) {
        this.networkId = networkId;
        this.networkMagic = getQedNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
    }
    static async create(proverProvider: IQedUserProverProvider, networkId: NetworkId, privateKeyHex: string) {
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
        if (contractCallArgs instanceof Array) {
            return this.prover.execContractCall(pk_hash, contractCallArgs);
        }
        return this.prover.execContractCall(pk_hash, [contractCallArgs]);
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

    async registerUser(privateKeyHex: string): Promise<string> {
        return this.prover.registerUser(privateKeyHex);
    }
}

export { QedMemoryTransactionSigner };
