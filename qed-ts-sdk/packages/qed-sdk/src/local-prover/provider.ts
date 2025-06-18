import { RpcConfig } from "./config";
import { WasmRpcServer } from "./qed_user_prover";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    DPNFunctionCircuitDefinition,
    ProofWithPublicInputs,
    QBCDeployContract,
    SubmitUserEndCapNonProofInput,
    ContractCallArgs,
    IQEDUserProverProvider,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";

export class QEDWasmUserProverProvider implements IQEDUserProverProvider {
    private wasmServer: WasmRpcServer;

    constructor(rpcConfigJson: RpcConfig) {
        const json = JSON.stringify(rpcConfigJson);
        this.wasmServer = new WasmRpcServer(json);
    }

    // Local proving operations
    async startSession(): Promise<string> {
        return this.wasmServer.start_session();
    }

    async proveContractCall(contractCallArg: ContractCallArgs): Promise<string> {
        const json = JSON.stringify(contractCallArg);
        return this.wasmServer.prove_contract_call_json(json);
    }

    async proveContractCalls(contractCallArgs: ContractCallArgs[]): Promise<string> {
        const json = JSON.stringify(contractCallArgs);
        return this.wasmServer.prove_contract_calls_json(json);
    }

    async signAndSubmit(): Promise<string> {
        return this.wasmServer.sign_and_submit();
    }

    // User operations
    async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.wasmServer.register_user(privateKey.toString());
    }

    async addUser(privateKey: PrivateKey): Promise<PublicKey> {
        return this.wasmServer.add_user(privateKey.toString());
    }

    async switchUser(pkHash: PublicKey): Promise<void> {
        this.wasmServer.switch_user(pkHash.toString());
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = this.wasmServer.get_zk_public_key_json(privateKey.toString());
        return JSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = this.wasmServer.get_random_keypair_json();
        return JSON.parse(json);
    }

    // Contract deployment
    async deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = JSON.stringify(circuitDefs);
        return this.wasmServer.deploy_contract_json(json);
    }

    async getDeployContractCmd(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract> {
        const json = JSON.stringify(circuitDefs);
        const resultJson = this.wasmServer.get_deploy_contract_cmd_json(json);
        return JSON.parse(resultJson);
    }

    // Signing and submission
    async getSigHash(networkMagic: bigint): Promise<QHashOut> {
        const result = this.wasmServer.get_sighash(networkMagic);
        return result;
    }

    async getZKSignature(sighash: QHashOut): Promise<ProofWithPublicInputs> {
        const json = this.wasmServer.get_zk_signature_json(sighash.toString());
        return JSON.parse(json);
    }

    async getEndCapProof(signatureProof: ProofWithPublicInputs): Promise<ProofWithPublicInputs> {
        const json = JSON.stringify(signatureProof);
        const resultJson = this.wasmServer.get_end_cap_proof_json(json);
        return JSON.parse(resultJson);
    }

    async getUserECInput(): Promise<SubmitUserEndCapNonProofInput> {
        const json = this.wasmServer.get_user_ec_input_json();
        return JSON.parse(json);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return this.wasmServer.ping(message);
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return this.wasmServer.get_result(id.toString());
    }
}
