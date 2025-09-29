import { WebProverConfig } from "./config";
import { WasmRpcServer } from "./qed_prover";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQedUserProverProvider,
    QBCDeployContract,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { JobInfo, ZKPublicKeyInfo } from "../types";
import { QedJSON } from "../utils";

export class QedWasmUserProverProvider implements IQedUserProverProvider {
    private wasmServer: WasmRpcServer;

    constructor(rpcConfigJson: WebProverConfig) {
        const json = QedJSON.stringify(rpcConfigJson);
        this.wasmServer = new WasmRpcServer(json);
    }

    async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        const json = QedJSON.stringify(contractCallArg);
        return this.wasmServer.exec_contract_call_json(pkHash, json);
    }

    async getClaimRewardsCallArgs(pkHash: PublicKey, checkpointId: bigint, jobInfos: JobInfo[]): Promise<ContractCallArgs[]> {
        const json = QedJSON.stringify(jobInfos);
        const contractCallArgs = await this.wasmServer.get_claim_rewards_call_args_json(pkHash, checkpointId, json);
        return QedJSON.parse(contractCallArgs) as ContractCallArgs[];
    }

    async claimRewards(pkHash: PublicKey, checkpointId: bigint, jobInfos: JobInfo[]): Promise<string> {
        const json = QedJSON.stringify(jobInfos);
        return this.wasmServer.claim_rewards_json(pkHash, checkpointId, json);
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return this.wasmServer.start_session(pkHash);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const json = QedJSON.stringify(contractCallArg);
        return this.wasmServer.prove_contract_call_json(pkHash, json);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const json = QedJSON.stringify(contractCallArgs);
        return this.wasmServer.prove_contract_calls_json(pkHash, json);
    }

    async signAndSubmit(pkHash: PublicKey): Promise<string> {
        return this.wasmServer.sign_and_submit(pkHash);
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.wasmServer.register_user(privateKey.toString());
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.wasmServer.add_user(privateKey.toString());
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await this.wasmServer.get_zk_public_key_json(privateKey.toString());
        return QedJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await this.wasmServer.get_random_keypair_json();
        return QedJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = QedJSON.stringify(circuitDefs);
        return this.wasmServer.deploy_contract_json(deployer, json);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = QedJSON.stringify(circuitDefs);
        const resultJson = await this.wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return QedJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.wasmServer.get_result(id.toString());
    }
}
