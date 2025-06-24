import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQedUserProverProvider,
    ProofWithPublicInputs,
    QBCDeployContract,
    QedUserProverRPCCommand,
    SubmitUserEndCapNonProofInput,
    WalletKeyPair,
} from "./types";

import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { IHTTPClient } from "../http";
import { BaseProvider } from "../provider";
import { ZKPublicKeyInfo } from "../types";
import { waitMs } from "../utils";

class QedRPCUserProverProvider extends BaseProvider implements IQedUserProverProvider {
    constructor(url: string, httpClient?: IHTTPClient) {
        super(url, httpClient);
    }

    async getResultFinal(hash: Promise<string>, maxAttempts: number, delay: number) {
        const resolvedHash = await hash;
        for (let i = 0; i < maxAttempts; i++) {
            try {
                return await this.getResult(resolvedHash);
            } catch (e) {
                console.log("Error in RPC call: " + e);
            }
            await waitMs(delay);
        }
        throw new Error("Result not found after " + maxAttempts + " attempts");
    }

    // Local proving operations
    async execContractCall(pk_hash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.ExecContractCall, [pk_hash, contractCallArg]);
    }

    async startSession(pk_hash: string): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.StartSession, [pk_hash]);
    }

    async proveContractCall(pk_hash: string, contractCallArg: ContractCallArgs): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.ProveContractCall, [pk_hash, contractCallArg]);
    }

    async proveContractCalls(pk_hash: string, contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.ProveContractCalls, [pk_hash, contractCallArgs]);
    }

    async signAndSubmit(pk_hash: string): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.SignAndSubmit, [pk_hash]);
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.rpc<QHashOut>(QedUserProverRPCCommand.RegisterUser, [privateKey]);
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.rpc<PublicKey>(QedUserProverRPCCommand.AddUser, [privateKey]);
    }

    // async switchUser(pkHash: PublicKey): Promise<void> {
    //     return this.rpc<void>(QedUserProverRPCCommand.SwitchUser, [pkHash]);
    // }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        return this.rpc<ZKPublicKeyInfo>(QedUserProverRPCCommand.GetZKPublicKey, [privateKey]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.rpc<WalletKeyPair>(QedUserProverRPCCommand.GetRandomKeypair, []);
    }

    // Contract deployment
    async deployContract(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.DeployContract, [deployer, circuitDefs]);
    }

    async getDeployContractCmd(
        deployer: string,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        return this.rpc<QBCDeployContract>(QedUserProverRPCCommand.GetDeployContractCmd, [deployer, circuitDefs]);
    }

    // Signing and submission
    // async getSigHash(networkMagic: bigint): Promise<QHashOut> {
    //     return this.rpc<QHashOut>(QedUserProverRPCCommand.GetSigHash, [networkMagic]);
    // }

    // async getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs> {
    //     return this.rpc<ProofWithPublicInputs>(QedUserProverRPCCommand.GetZKSignature, [sighash]);
    // }

    // async getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs> {
    //     return this.rpc<ProofWithPublicInputs>(QedUserProverRPCCommand.GetEndCapProof, [signatureProof]);
    // }

    // async getUserECInput(): Promise<SubmitUserEndCapNonProofInput> {
    //     return this.rpc<SubmitUserEndCapNonProofInput>(QedUserProverRPCCommand.GetUserECInput, []);
    // }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.rpc<string>(QedUserProverRPCCommand.Ping, [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.rpc<U8Bytes>(QedUserProverRPCCommand.GetResult, [id]);
    }
}

export { QedRPCUserProverProvider };
