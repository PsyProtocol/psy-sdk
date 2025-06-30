import { WebProverConfig } from "./config";
import { initSync, WasmRpcServer } from "./qed_user_prover";
import { wasmBinary } from "./wasm-binary";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQedUserProverProvider,
    QBCDeployContract,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { QedJSON } from "../utils";


// Synchronous WASM initialization function
function initWasmSync(): void {
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync(wasmBinary);
        
        console.log('WASM initialized synchronously from binary data');
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        throw error;
    }
}

export class QedWasmWebProverProvider implements IQedUserProverProvider {
    private static wasmServer: WasmRpcServer;

    constructor(rpcConfigJson: WebProverConfig) {
        const json = QedJSON.stringify(rpcConfigJson);
        if (!QedWasmWebProverProvider.wasmServer) {
            initWasmSync();
            QedWasmWebProverProvider.wasmServer = new WasmRpcServer(json);
        }
    }


    async execContractCall(pkHash: string, contractCallArg: ContractCallArgs[]): Promise<string> {
        const json = QedJSON.stringify(contractCallArg);
        return QedWasmWebProverProvider.wasmServer.exec_contract_call_json(pkHash, json);
    }

    // Local proving operations
    async startSession(pkHash: PublicKey): Promise<string> {
        return QedWasmWebProverProvider.wasmServer.start_session(pkHash);
    }

    async proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string> {
        const json = QedJSON.stringify(contractCallArg);
        return QedWasmWebProverProvider.wasmServer.prove_contract_call_json(pkHash, json);
    }

    async proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string> {
        const json = QedJSON.stringify(contractCallArgs);
        return QedWasmWebProverProvider.wasmServer.prove_contract_calls_json(pkHash, json);
    }

    async signAndSubmit(pkHash: PublicKey): Promise<string> {
        return QedWasmWebProverProvider.wasmServer.sign_and_submit(pkHash);
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return QedWasmWebProverProvider.wasmServer.register_user(privateKey.toString());
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return QedWasmWebProverProvider.wasmServer.add_user(privateKey.toString());
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await QedWasmWebProverProvider.wasmServer.get_zk_public_key_json(privateKey.toString());
        return QedJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await QedWasmWebProverProvider.wasmServer.get_random_keypair_json();
        return QedJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = QedJSON.stringify(circuitDefs);
        return QedWasmWebProverProvider.wasmServer.deploy_contract_json(deployer, json);
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = QedJSON.stringify(circuitDefs);
        const resultJson = await QedWasmWebProverProvider.wasmServer.get_deploy_contract_cmd_json(deployer, json);
        return QedJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return QedWasmWebProverProvider.wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return QedWasmWebProverProvider.wasmServer.get_result(id.toString());
    }
}
