import { IQedUserWallet } from "./types";
import { PrivateKey, PublicKey, QHashOut } from "../core";
import {
    IQEDUserProverProvider,
    WalletKeyPair,
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
} from "../local-prover-rpc";
import { ZKPublicKeyInfo } from "../types";

class QedUserWallet implements IQedUserWallet {
    prover: IQEDUserProverProvider;
    privateKey: PrivateKey;

    constructor(prover: IQEDUserProverProvider, privateKeyHex: PrivateKey) {
        this.prover = prover;
        this.privateKey = privateKeyHex;
    }

    async registerUser(privateKey: PrivateKey): Promise<QHashOut> {
        return this.prover.registerUser(privateKey);
    }

    async getZKPublicKey(): Promise<ZKPublicKeyInfo> {
        return this.prover.getZKPublicKey(this.privateKey);
    }

    async importPrivateKey(privateKey: PrivateKey): Promise<PublicKey> {
        return this.prover.addUser(privateKey);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.prover.getRandomKeypair();
    }

    async deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        await this.prover.startSession();
        const publicKey = await this.prover.addUser(this.privateKey);
        await this.prover.switchUser(publicKey);
        await this.prover.deployContract(circuitDefs);
        return this.prover.signAndSubmit();
    }

    async contractCall(contractCallArgs: ContractCallArgs[]): Promise<string> {
        await this.prover.startSession();
        const publicKey = await this.prover.addUser(this.privateKey);
        await this.prover.switchUser(publicKey);
        await this.prover.proveContractCalls(contractCallArgs);
        return this.prover.signAndSubmit();
    }
}

export { QedUserWallet };
