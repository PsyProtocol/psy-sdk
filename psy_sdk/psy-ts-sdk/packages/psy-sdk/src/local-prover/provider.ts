import { WasmRpcServer } from "./psy_prover";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    SignData,
    SignType,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyJSON } from "../utils";
import { PsyNetworkConfig } from "../config";

export class PsyWasmUserProverProvider implements IPsyUserProverProvider {
    private wasmServer: WasmRpcServer;

    constructor(rpcConfigJson: PsyNetworkConfig) {
        const json = PsyJSON.stringify(rpcConfigJson);
        this.wasmServer = new WasmRpcServer(json);
    }

    async execContractCall(pkHash: string, callData: ContractCallData): Promise<string> {
        const json = PsyJSON.stringify(callData);
        return this.wasmServer.exec_contract_call_json(pkHash, json);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        // const json = PsyJSON.stringify(jobInfos);
        const contractCallArgs = await this.wasmServer.get_claim_rewards_call_args_json(jobInfos);
        return PsyJSON.parse(contractCallArgs) as ContractCallArgs[];
    }

    async claimRewards(pkHash: PublicKey, jobInfos: string): Promise<string> {
        // const json = PsyJSON.stringify(jobInfos);
        return this.wasmServer.claim_rewards_json(pkHash, jobInfos);
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return this.wasmServer.start_session(pkHash);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const json = PsyJSON.stringify(contractCallArg);
        return this.wasmServer.prove_contract_call_json(pkHash, json);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const json = PsyJSON.stringify(contractCallArgs);
        return this.wasmServer.prove_contract_calls_json(pkHash, json);
    }

    async signAndSubmit(pkHash: PublicKey, signData?: SignData): Promise<string> {
        const signDataJson = signData ? PsyJSON.stringify(signData) : null;
        return this.wasmServer.sign_and_submit(pkHash, signDataJson);
    }


    // User operations
    async registerUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.wasmServer.register_user(privateKey.toString(), signType);
    }

    async addUser(privateKey: PrivateKey, signType: SignType): Promise<PublicKey> {
        return this.wasmServer.add_user(privateKey.toString(), signType);
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await this.wasmServer.get_zk_public_key_json(privateKey.toString());
        return PsyJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await this.wasmServer.get_random_keypair_json();
        return PsyJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[], abi_json: string): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        return this.wasmServer.deploy_contract_json(deployer, json, abi_json);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = PsyJSON.stringify(circuitDefs);
        const resultJson = await this.wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return PsyJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.wasmServer.get_result(id.toString());
    }
}
