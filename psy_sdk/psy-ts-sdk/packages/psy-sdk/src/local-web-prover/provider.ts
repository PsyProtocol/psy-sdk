import { WebProverConfig } from "./config";
import { initSync, WasmRpcServer } from "./psy_prover";
import { wasmBinary } from "./wasm-binary";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    SignData,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo, JobInfo } from "../types";
import { PsyJSON } from "../utils";


// Synchronous WASM initialization function
export function initWasmSync(): void {
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync(wasmBinary);
        
        console.log('WASM initialized synchronously from binary data');
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        throw error;
    }
}

export class PsyWasmWebProverProvider implements IPsyUserProverProvider {
    static wasmServer: WasmRpcServer;

    constructor(rpcConfigJson: WebProverConfig) {
        const json = PsyJSON.stringify(rpcConfigJson);
        if (!PsyWasmWebProverProvider.wasmServer) {
            const now = new Date().getTime();
            initWasmSync();
            PsyWasmWebProverProvider.wasmServer = new WasmRpcServer(json);
            console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
        }
    }

    // async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
    //     const now = new Date().getTime();
    //     const json = PsyJSON.stringify(contractCallArg);
    //     const result = await PsyWasmWebProverProvider.wasmServer.exec_contract_call_json(pkHash, json);
    //     console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
    //     return result;
    // }

    async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        const now = new Date().getTime();
        
        // await this.startSession(pkHash);
        // const result = await this.proveContractCalls(pkHash, contractCallArg);
        // await this.signAndSubmit(pkHash);

        const json = PsyJSON.stringify(contractCallArg);
        const result = await PsyWasmWebProverProvider.wasmServer.exec_contract_call_json(pkHash, json);

        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async execContractCallWithSignData(pkHash: string, contractCallArg: ContractCallArgs[], signData: SignData|null|undefined): Promise<QHashOut> {
        const now = new Date().getTime();

        const json = PsyJSON.stringify(contractCallArg);
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const result = await PsyWasmWebProverProvider.wasmServer.exec_contract_call_with_sign_data_json(pkHash, json, signDataJson);

        console.log(`execContractCallWithSignData in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        const result = await PsyWasmWebProverProvider.wasmServer.get_claim_rewards_call_args_json(jobInfos);
        console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        const contractCallArgs = PsyJSON.parse(result) as ContractCallArgs[];
        return contractCallArgs;
    }

    async claimRewards(pkHash: string, jobInfos: string): Promise<string> {
        const now = new Date().getTime();
        // const json = PsyJSON.stringify(jobInfos);
        const result = await PsyWasmWebProverProvider.wasmServer.claim_rewards_json(pkHash, jobInfos);
        console.log(`claimRewards in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return PsyWasmWebProverProvider.wasmServer.start_session(pkHash);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArg);
        const result = await PsyWasmWebProverProvider.wasmServer.prove_contract_call_json(pkHash, json);
        console.log(`proveContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(contractCallArgs);
        const result = await PsyWasmWebProverProvider.wasmServer.prove_contract_calls_json(pkHash, json);
        console.log(`proveContractCalls in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async signAndSubmit(pkHash: PublicKey): Promise<string> {
        const now = new Date().getTime();
        const result = await PsyWasmWebProverProvider.wasmServer.sign_and_submit(pkHash);
        console.log(`signAndSubmit in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async signAndSubmitWithData(pkHash: PublicKey, signData: SignData|null|undefined): Promise<QHashOut> {
        const now = new Date().getTime();
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        const result = await PsyWasmWebProverProvider.wasmServer.sign_and_submit_with_sign_data(pkHash, signDataJson);
        console.log(`signAndSubmitWithData in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        const now = new Date().getTime();
        const result = await PsyWasmWebProverProvider.wasmServer.register_user(privateKey.toString());
        console.log(`registerUser in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async registerUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        const now = new Date().getTime();
        const result = await PsyWasmWebProverProvider.wasmServer.register_user_with_type(privateKey.toString(), signType, fingerprint);
        console.log(`registerUserWithType in ${(new Date().getTime() - now) / 1000} seconds`);
        return result;
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return PsyWasmWebProverProvider.wasmServer.add_user(privateKey.toString());
    }

    async addUserWithType(privateKey: PrivateKey, signType: string, fingerprint: string|null|undefined): Promise<PublicKey> {
        return PsyWasmWebProverProvider.wasmServer.add_user_with_type(privateKey.toString(), signType, fingerprint);
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await PsyWasmWebProverProvider.wasmServer.get_zk_public_key_json(privateKey.toString());
        return PsyJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await PsyWasmWebProverProvider.wasmServer.get_random_keypair_json();
        return PsyJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.wasmServer.deploy_contract_json(deployer, json);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = PsyJSON.stringify(circuitDefs);
        const resultJson = await PsyWasmWebProverProvider.wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return PsyJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return PsyWasmWebProverProvider.wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return PsyWasmWebProverProvider.wasmServer.get_result(id.toString());
    }
}
