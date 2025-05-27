import { FetchHTTPClient } from "../http/fetchClient";
import { IHTTPClient } from "../http/types";
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
import { Hash256 } from "../rpc/baseTypes";
import { CityJSON, waitMs } from "../utils";

class QEDRPCUserProverProvider implements IQEDUserProverProvider {
    httpClient: IHTTPClient;
    url: string;

    constructor(url: string, httpClient?: IHTTPClient) {
        this.httpClient = httpClient || new FetchHTTPClient();
        this.url = url;
    }

    async rpc<T>(method: string, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: CityJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });

        if (response.statusCode >= 400) {
            throw new Error("Error in RPC call: " + CityJSON.stringify(response.body));
        }
        const result = CityJSON.parse(response.body);
        if (result.error) {
            throw new Error("Error in RPC call: " + CityJSON.stringify(result.error));
        } else {
            return result.body.result as T;
        }
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
    async registerUser(privateKey: Hash256): Promise<Hash256> {
        return this.rpc<Hash256>(QEDUserProverRPCCommand.RegisterUser, [privateKey]);
    }

    async addUser(privateKey: Hash256): Promise<Hash256> {
        return this.rpc<Hash256>(QEDUserProverRPCCommand.AddUser, [privateKey]);
    }

    async switchUser(pkHash: Hash256): Promise<void> {
        return this.rpc<void>(QEDUserProverRPCCommand.SwitchUser, [pkHash]);
    }

    async getZKPublicKey(privateKey: Hash256): Promise<ZKPublicKeyInfo> {
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
    async getSigHash(networkMagic: bigint): Promise<Hash256> {
        return this.rpc<Hash256>(QEDUserProverRPCCommand.GetSigHash, [networkMagic]);
    }

    async getZKSignature(sighash: Hash256): Promise<ProofWithPublicInputs> {
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

    async getResult(id: Hash256): Promise<Uint8Array | string> {
        return this.rpc<Uint8Array | string>(QEDUserProverRPCCommand.GetResult, [id]);
    }
}

export { QEDRPCUserProverProvider };
