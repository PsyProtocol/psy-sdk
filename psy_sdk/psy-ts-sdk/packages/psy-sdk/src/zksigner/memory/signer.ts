import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ContractCallArgs, DPNFunctionCircuitDefinition, IPsyUserProverProvider } from "../../local-prover-rpc";
import { JobInfo } from "../../types";
import { IPsyTransactionSigner, TPsyTransactionSignerAbility } from "../types";

class PsyMemoryTransactionSigner implements IPsyTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    signType: string;
    fingerprint?: string;
    prover: IPsyUserProverProvider;
    private constructor(
        proverProvider: IPsyUserProverProvider,
        networkId: NetworkId,
        publicKeyHex: string,
        privateKeyHex: string,
        signType: string,
        fingerprint?: string
    ) {
        this.networkId = networkId;
        this.networkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
        this.signType = signType;
        this.fingerprint = fingerprint;
    }
    static async create(proverProvider: IPsyUserProverProvider, networkId: NetworkId, privateKeyHex: string, signType: string, fingerprint?: string) {
        const publicKeyHex = await proverProvider.addUserWithType(privateKeyHex, signType, fingerprint);
        return new PsyMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex, signType, fingerprint);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }

    getSignType(): Promise<string> {
        return Promise.resolve(this.signType);
    }

    getFingerprint(): Promise<string|null|undefined> {
        return Promise.resolve(this.fingerprint);
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

    getAbilities(): TPsyTransactionSignerAbility[] {
        return ["sign-hash", "export-private-key-hex"];
    }

    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }

    async registerUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string> {
        return this.prover.registerUserWithType(privateKeyHex, signType, fingerprint);
    }

    async addUser(privateKeyHex: string, signType: string, fingerprint?: string): Promise<string> {
        return this.prover.addUserWithType(privateKeyHex, signType, fingerprint);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        return this.prover.getClaimRewardsCallArgs(jobInfos);
    }

    async claimRewards(pk_hash: string, jobInfos: string): Promise<string> {
        return this.prover.claimRewards(pk_hash, jobInfos);
    }
}

export { PsyMemoryTransactionSigner };
