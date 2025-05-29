import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQEDUserProverProvider,
    ProofWithPublicInputs,
    QBCDeployContract,
    QEDUserProverRPCCommand,
    SubmitUserEndCapNonProofInput,
    WalletKeyPair,
    ZKPublicKeyInfo,
} from "./qedTypes";
import { IHTTPClient } from "../http/types";
import { BaseProvider } from "../provider";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../rpc/baseTypes";
import { waitMs } from "../utils";

class QEDRPCUserProverProvider extends BaseProvider implements IQEDUserProverProvider {
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
    async startSession(): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.StartSession, []);
    }

    async proveContractCall(contractCallArg: ContractCallArgs): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.ProveContractCall, [contractCallArg]);
    }

    async proveContractCalls(contractCallArgs: ContractCallArgs[]): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.ProveContractCalls, [contractCallArgs]);
    }

    async signAndSubmit(): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.SignAndSubmit, []);
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<QHashOut> {
        return this.rpc<QHashOut>(QEDUserProverRPCCommand.RegisterUser, [privateKey]);
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.rpc<PublicKey>(QEDUserProverRPCCommand.AddUser, [privateKey]);
    }

    async switchUser(pkHash: PublicKey): Promise<void> {
        return this.rpc<void>(QEDUserProverRPCCommand.SwitchUser, [pkHash]);
    }

    async getZKPublicKey(privateKey: QHashOut): Promise<ZKPublicKeyInfo> {
        return this.rpc<ZKPublicKeyInfo>(QEDUserProverRPCCommand.GetZKPublicKey, [privateKey]);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        return this.rpc<WalletKeyPair>(QEDUserProverRPCCommand.GetRandomKeypair, []);
    }

    // Contract deployment
    async deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.DeployContract, [circuitDefs]);
    }

    async getDeployContractCmd(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract> {
        return this.rpc<QBCDeployContract>(QEDUserProverRPCCommand.GetDeployContractCmd, [circuitDefs]);
    }

    // Signing and submission
    async getSigHash(networkMagic: bigint): Promise<QHashOut> {
        return this.rpc<QHashOut>(QEDUserProverRPCCommand.GetSigHash, [networkMagic]);
    }

    async getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs> {
        return this.rpc<ProofWithPublicInputs>(QEDUserProverRPCCommand.GetZKSignature, [sighash]);
    }

    async getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs> {
        return this.rpc<ProofWithPublicInputs>(QEDUserProverRPCCommand.GetEndCapProof, [signatureProof]);
    }

    async getUserECInput(): Promise<SubmitUserEndCapNonProofInput> {
        return this.rpc<SubmitUserEndCapNonProofInput>(QEDUserProverRPCCommand.GetUserECInput, []);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.rpc<string>(QEDUserProverRPCCommand.Ping, [message]);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.rpc<U8Bytes>(QEDUserProverRPCCommand.GetResult, [id]);
    }
}

export { QEDRPCUserProverProvider };
