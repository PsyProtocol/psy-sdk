import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    PsyUserProverRPCCommand,
    SignData,
    WalletKeyPair,
} from "./types";

import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { IHTTPClient } from "../http";
import { BaseProvider } from "../provider";
import { JobInfo, ZKPublicKeyInfo } from "../types";
import { waitMs } from "../utils";

class PsyRPCUserProverProvider extends BaseProvider implements IPsyUserProverProvider {
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
        return this.rpc<string>(PsyUserProverRPCCommand.ExecContractCall, [pk_hash, contractCallArg]);
    }

    async execContractCallWithSignData(pk_hash: string, contractCallArg: ContractCallArgs[], signData: SignData|null|undefined): Promise<QHashOut> {
        return this.rpc<QHashOut>(PsyUserProverRPCCommand.ExecContractCallWithSignData, [pk_hash, contractCallArg, signData]);
    }

    async startSession(pk_hash: string): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.StartSession, [pk_hash]);
    }

    async proveContractCall(pk_hash: string, contractCallArg: ContractCallArgs): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.ProveContractCall, [pk_hash, contractCallArg]);
    }

    async proveContractCalls(pk_hash: string, contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.ProveContractCalls, [pk_hash, contractCallArgs]);
    }

    async signAndSubmit(pk_hash: string): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.SignAndSubmit, [pk_hash]);
    }

    async signAndSubmitWithData(pk_hash: string, signData: SignData|null|undefined): Promise<QHashOut> {
        return this.rpc<QHashOut>(PsyUserProverRPCCommand.SignAndSubmitWithData, [pk_hash, signData]);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(pkHash: PublicKey, jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.rpc<QHashOut>(PsyUserProverRPCCommand.RegisterUser, [privateKey]);
    }

    async registerUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        return this.rpc<PublicKey>(PsyUserProverRPCCommand.RegisterUserWithType, [privateKey, signType, fingerprint]);
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.rpc<PublicKey>(PsyUserProverRPCCommand.AddUser, [privateKey]);
    }

    async addUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        return this.rpc<PublicKey>(PsyUserProverRPCCommand.AddUserWithType, [privateKey, signType, fingerprint]);
    }

    // async switchUser(pkHash: PublicKey): Promise<void> {
    //     return this.rpc<void>(PsyUserProverRPCCommand.SwitchUser, [pkHash]);
    // }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        return this.rpc<ZKPublicKeyInfo>(PsyUserProverRPCCommand.GetZKPublicKey, [privateKey]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.rpc<WalletKeyPair>(PsyUserProverRPCCommand.GetRandomKeypair, []);
    }

    // Contract deployment
    async deployContract(deployer: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.DeployContract, [deployer, circuitDefs]);
    }

    async getDeployContractCmd(
        deployer: string,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        return this.rpc<QBCDeployContract>(PsyUserProverRPCCommand.GetDeployContractCmd, [deployer, circuitDefs]);
    }

    // Signing and submission
    // async getSigHash(networkMagic: bigint): Promise<QHashOut> {
    //     return this.rpc<QHashOut>(PsyUserProverRPCCommand.GetSigHash, [networkMagic]);
    // }

    // async getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs> {
    //     return this.rpc<ProofWithPublicInputs>(PsyUserProverRPCCommand.GetZKSignature, [sighash]);
    // }

    // async getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs> {
    //     return this.rpc<ProofWithPublicInputs>(PsyUserProverRPCCommand.GetEndCapProof, [signatureProof]);
    // }

    // async getUserECInput(): Promise<SubmitUserEndCapNonProofInput> {
    //     return this.rpc<SubmitUserEndCapNonProofInput>(PsyUserProverRPCCommand.GetUserECInput, []);
    // }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.Ping, [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.rpc<U8Bytes>(PsyUserProverRPCCommand.GetResult, [id]);
    }
}

export { PsyRPCUserProverProvider };
