import {
    ContractCallArgs,
    ContractCallData,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    PsyUserProverRPCCommand,
    SignData,
    SignType,
    WalletKeyPair,
} from "./types";

import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { IHTTPClient } from "../http";
import { BaseProvider } from "../provider";
import { ZKPublicKeyInfo } from "../types";
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
    async execContractCall(pk_hash: string, callData: ContractCallData): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.ExecContractCall, [pk_hash, callData]);
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

    async signAndSubmit(pk_hash: string, signData?: SignData): Promise<string> {
        return this.rpc<string>(PsyUserProverRPCCommand.SignAndSubmit, [pk_hash, signData]);
    }

    async getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(_pkHash: PublicKey, _jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    // User operations
    async registerUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.rpc<PublicKey>(PsyUserProverRPCCommand.RegisterUser, [privateKey, signType]);
    }

    async addUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.rpc<PublicKey>(PsyUserProverRPCCommand.AddUser, [privateKey, signType]);
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
