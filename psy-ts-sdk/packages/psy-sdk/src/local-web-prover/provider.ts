import { initSync, WasmRpcServer, WasmPsyConfig, WasmPsyConfigBuilder, WasmConstants } from "./psy_prover";
import { wasmBinary } from "./wasm-binary";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import {
    ContractCallArgs,
    ContractCallData,
    ClaimBatchItem,
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    SignType,
    TxMetadata,
    WalletKeyPair,
} from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyJSON } from "../utils";
import { PsyNetworkConfig } from "../config";

let isWasmInitialized = false;

// Synchronous WASM initialization function
export function initWasmSync(): void {
    if (isWasmInitialized) {
        return;
    }

    try {
        // Initialize synchronously with pre-compiled binary data
        initSync({ module: wasmBinary });
        isWasmInitialized = true;

        console.log("WASM initialized synchronously from binary data");
    } catch (error) {
        console.error("Failed to initialize WASM:", error);
        throw error;
    }
}

export class PsyWasmWebProverProvider implements IPsyUserProverProvider {
    private static wasmServer: Promise<WasmRpcServer> | null = null;
    private static wasmServerConfigJson: string | null = null;
    private static wasmCallQueue: Promise<void> = Promise.resolve();

    constructor(rpcConfigJson: PsyNetworkConfig) {
        const json = PsyJSON.stringify(rpcConfigJson);
        console.log(`WASM init with config: ${json}`);
        void PsyWasmWebProverProvider.ensureWasmServer(json);
    }

    static ensureWasmServer(rpcConfigJson: PsyNetworkConfig | string): Promise<WasmRpcServer> {
        const json = typeof rpcConfigJson === "string" ? rpcConfigJson : PsyJSON.stringify(rpcConfigJson);
        if (!this.wasmServer) {
            const now = new Date().getTime();
            initWasmSync();
            this.wasmServer = Promise.resolve(
                new WasmRpcServer(json) as unknown as WasmRpcServer | Promise<WasmRpcServer>
            );
            this.wasmServerConfigJson = json;
            this.wasmServer.then(() => {
                console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
            });
        } else if (this.wasmServerConfigJson !== json) {
            console.warn(
                "WASM RPC server is a singleton; ignoring a different config and reusing the existing server."
            );
        }

        return this.wasmServer;
    }

    static runWasmServerCall<T>(callback: (server: WasmRpcServer) => T | Promise<T>): Promise<T> {
        const run = async (): Promise<T> => {
            if (!this.wasmServer) {
                throw new Error("WASM RPC server is not initialized");
            }

            return callback(await this.wasmServer);
        };

        const result = this.wasmCallQueue.then(run, run);
        this.wasmCallQueue = result.then(
            () => undefined,
            () => undefined,
        );
        return result;
    }

    async execContractCall(pkHash: string, callData: ContractCallData): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_json(pkHash, json)
        );
        console.log(`execContractCall in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    async execContractCallWithoutProof(pkHash: string, callData: ContractCallData): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(callData);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_without_proof_json(pkHash, json)
        );
        console.log(`execContractCallWithoutProof in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    async claimBatch(pkHash: string, claims: ClaimBatchItem[]): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_claim_batch_json(pkHash, json)
        );
        console.log(`claimBatch in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    async claimBatchWithoutProof(pkHash: string, claims: ClaimBatchItem[]): Promise<TxMetadata> {
        const now = new Date().getTime();
        const json = PsyJSON.stringify(claims);
        const result = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_claim_batch_without_proof_json(pkHash, json)
        );
        console.log(`claimBatchWithoutProof in ${(new Date().getTime() - now) / 1000} seconds`);
        return PsyJSON.parse(result) as TxMetadata;
    }

    // ===================================================================
    // MODE-A (web / MetaMask) external-signature authorization. Delegates to the
    // WASM external-signature methods. The web wallet reuses the EXISTING
    // secp256k1 account type; the signature is supplied from OUTSIDE (MetaMask
    // eth_sign over the Psy sighash), so no private key is held here.
    //
    // signatureHex format (every Mode-A method): compressedPubkey(33) ‖ r(32) ‖
    // s(32) = 97 bytes hex (optional `0x` prefix); leading byte 0x02/0x03.
    // ===================================================================

    /** MODE-A step 1 (contract call): prime the session and return the 32-byte
     * sighash (hex) MetaMask must eth_sign. */
    async getSigHash(pkHash: string, callData: ContractCallData): Promise<string> {
        const json = PsyJSON.stringify(callData);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_sig_hash(pkHash, json)
        );
    }

    /** MODE-A step 3 (contract call): submit authorized solely by an external
     * eth_sign signature. Returns the tx end-user-leaf hash. */
    async execContractCallWithExternalSignature(pkHash: string, callData: ContractCallData, signatureHex: string): Promise<string> {
        const json = PsyJSON.stringify(callData);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_with_external_signature(pkHash, json, signatureHex)
        );
    }

    /** MODE-A core primitive: register a secp256k1 PUBLIC key as a Psy account
     * with no held key, authorized by an external eth_sign. Returns JSON
     * `{ pk_hash, user_id }` (user_id null until registration lands on-chain). */
    async registerUserWithExternalSignature(publicKeyHex: string, signatureHex: string): Promise<{ pk_hash: string; user_id: string | null }> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.register_user_with_external_signature(publicKeyHex, signatureHex)
        );
        return PsyJSON.parse(json);
    }

    /** MODE-A step 1 (claim): prime the claim session and return the sighash
     * (hex) MetaMask must eth_sign. */
    async getClaimSigHash(pkHash: string, claims: ClaimBatchItem[]): Promise<string> {
        const json = PsyJSON.stringify(claims);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_claim_sig_hash(pkHash, json)
        );
    }

    /** MODE-A step 3 (claim): submit a claim batch authorized solely by an
     * external eth_sign signature. Returns the tx end-user-leaf hash. */
    async claimBatchWithExternalSignature(pkHash: string, claims: ClaimBatchItem[], signatureHex: string): Promise<string> {
        const json = PsyJSON.stringify(claims);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.claim_batch_with_external_signature(pkHash, json, signatureHex)
        );
    }

    // ── MODE-A (MetaMask `personal_sign` / EIP-191) variants ──────────────────
    // Same shape as the *WithExternalSignature methods above, but the proof is
    // produced by the keccak-prefix circuit (ExternalEthPersonalSignUser). The
    // wallet `personal_sign`s the SAME sighash value getSigHash/getClaimSigHash
    // return; the EIP-191 keccak is recomputed in-circuit. Distinct identity from
    // the classic-secp path — register via registerUserWithExternalEthPersonalSignature.

    /** MODE-A (personal_sign) register: register a secp256k1 PUBLIC key under the
     * EIP-191 signature type, no held key. Returns `{ pk_hash, user_id }`. */
    async registerUserWithExternalEthPersonalSignature(publicKeyHex: string, signatureHex: string): Promise<{ pk_hash: string; user_id: string | null }> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.register_user_with_external_eth_personal_signature(publicKeyHex, signatureHex)
        );
        return PsyJSON.parse(json);
    }

    /** MODE-A (personal_sign) contract call: submit authorized solely by a
     * MetaMask personal_sign. Returns the tx end-user-leaf hash. */
    async execContractCallWithExternalEthPersonalSignature(pkHash: string, callData: ContractCallData, signatureHex: string): Promise<string> {
        const json = PsyJSON.stringify(callData);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.exec_contract_call_with_external_eth_personal_signature(pkHash, json, signatureHex)
        );
    }

    /** MODE-A (personal_sign) claim: submit a claim batch authorized solely by a
     * MetaMask personal_sign. Returns the tx end-user-leaf hash. */
    async claimBatchWithExternalEthPersonalSignature(pkHash: string, claims: ClaimBatchItem[], signatureHex: string): Promise<string> {
        const json = PsyJSON.stringify(claims);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.claim_batch_with_external_eth_personal_signature(pkHash, json, signatureHex)
        );
    }

    async getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]> {
        throw new Error("Method not implemented.");
    }

    async claimRewards(_pkHash: string, _jobInfos: string): Promise<string> {
        throw new Error("Method not implemented.");
    }

    // User operations
    async registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.register_user(privateKey.toString(), signType, fingerprint)
        );
    }

    async addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.add_user(privateKey.toString(), signType, fingerprint)
        );
    }

    async getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_zk_public_key_json(privateKey.toString())
        );
        return PsyJSON.parse(json);
    }

    async getRandomKeypair(): Promise<WalletKeyPair> {
        const json = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_random_keypair_json()
        );
        return PsyJSON.parse(json);
    }

    // Contract deployment
    async deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        const json = PsyJSON.stringify(circuitDefs);
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.deploy_contract_json(deployer, json)
        );
    }

    async getDeployContractCmd(
        deployer: PublicKey,
        circuitDefs: DPNFunctionCircuitDefinition[]
    ): Promise<QBCDeployContract> {
        const json = PsyJSON.stringify(circuitDefs);
        const resultJson = await PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_deploy_contract_cmd_json(deployer, json)
        );
        return PsyJSON.parse(resultJson);
    }

    // Utility methods
    async ping(message: string): Promise<string> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.ping(message)
        );
    }

    async getResult(id: QHashOut): Promise<U8Bytes> {
        return PsyWasmWebProverProvider.runWasmServerCall((server) =>
            server.get_result(id.toString())
        );
    }
}

export class PsyWasmConstantsProvider {
    private static _cache: Record<string, any> | null = null;

    static getAll(): Record<string, any> {
        if (!this._cache) {
            try {
                this._cache = JSON.parse(WasmConstants.getAllConstants());
            } catch (err) {
                console.warn("[PsyWasmConstantsProvider] parse error:", err);
                this._cache = {};
            }
        }
        return this._cache!;
    }

    static get<T = any>(key: keyof typeof WasmConstants | string): T {
        if (key in WasmConstants) {
            return (WasmConstants as any)[key] as T;
        }
        return this.getAll()[key as string] as T;
    }

    static getRawJson(): string {
        return WasmConstants.getAllConstants();
    }

    static refresh(): void {
        this._cache = null;
    }

    static get globalUserTreeHeight(): number {
        return WasmConstants.global_user_tree_height;
    }

    static get coordinatorUserTreeHeight(): number {
        return WasmConstants.coordinator_user_tree_height;
    }

    static get realmUserTreeHeight(): number {
        return WasmConstants.realm_user_tree_height;
    }

    static get groupRealmHeight(): number {
        return WasmConstants.group_realm_height;
    }

    static get usersPerRealm(): bigint {
        return WasmConstants.users_per_realm;
    }

    static get nativeCurrency(): string {
        return WasmConstants.native_currency;
    }

    static get nativeCurrencyName(): string {
        return WasmConstants.native_currency_name;
    }

    static get nativeCurrencyDecimal(): number {
        return WasmConstants.native_currency_decimal;
    }

    static get registerUserFee(): bigint {
        return WasmConstants.register_user_fee;
    }

    static get deployContractFee(): bigint {
        return WasmConstants.deploy_contract_fee;
    }

    static get gutaFee(): bigint {
        return WasmConstants.guta_fee;
    }

    static get currentNetwork(): string {
        return WasmConstants.current_network;
    }

    static get configPath(): string {
        return WasmConstants.config_path;
    }

    static get coordinatorRpcUrl(): string {
        return WasmConstants.coordinator_rpc_url;
    }

    static get realmRpcUrls(): string[] {
        return WasmConstants.realm_rpc_urls;
    }
}

export class PsyWasmConfigBuilderProvider {
    static wasmPsyConfigBuilder: WasmPsyConfigBuilder | null = null;
    static wasmPsyConfig: WasmPsyConfig | null = null;

    static initBuilder(): WasmPsyConfigBuilder {
        this.wasmPsyConfigBuilder = new WasmPsyConfigBuilder();
        return this.wasmPsyConfigBuilder;
    }

    static fromJson(json: string): WasmPsyConfigBuilder {
        const builder = new WasmPsyConfigBuilder();
        builder.json(json);
        this.wasmPsyConfigBuilder = builder;
        return builder;
    }

    static setNetwork(network: string): WasmPsyConfigBuilder {
        if (!this.wasmPsyConfigBuilder) this.initBuilder();
        this.wasmPsyConfigBuilder!.network(network);
        return this.wasmPsyConfigBuilder!;
    }

    static build(): WasmPsyConfig {
        if (!this.wasmPsyConfigBuilder)
            throw new Error("WasmPsyConfigBuilder not initialized");
        this.wasmPsyConfig = this.wasmPsyConfigBuilder.build();
        return this.wasmPsyConfig!;
    }

    static useNetwork(network: string): void {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        this.wasmPsyConfig.useNetwork(network);
    }

    static getCurrentNetwork(): string {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getCurrentNetwork();
    }

    static listNetworks(): string[] {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.listNetworks();
    }

    static getNetworkJson(network: string): string {
        if (!this.wasmPsyConfig)
            throw new Error("WasmPsyConfig not built yet. Call build() first.");
        return this.wasmPsyConfig.getNetworkJson(network);
    }
}
