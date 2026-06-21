import { WasmRpcServer, WasmPsyConfig, WasmPsyConfigBuilder, WasmConstants } from "./psy_prover";
import { PrivateKey, PublicKey, QHashOut, U8Bytes } from "../core";
import { ContractCallArgs, ContractCallData, ClaimBatchItem, DPNFunctionCircuitDefinition, IPsyUserProverProvider, QBCDeployContract, SignData, SignType, WalletKeyPair } from "../local-prover-rpc/types";
import { ZKPublicKeyInfo } from "../types";
import { PsyNetworkConfig } from "../config";
export declare function initWasmSync(): void;
export declare class PsyWasmWebProverProvider implements IPsyUserProverProvider {
    static wasmServer: WasmRpcServer | Promise<WasmRpcServer> | null;
    private static getWasmServer;
    constructor(rpcConfigJson: PsyNetworkConfig);
    execContractCall(pkHash: string, callData: ContractCallData): Promise<string>;
    claimBatch(pkHash: string, claims: ClaimBatchItem[]): Promise<string>;
    generateBatchClaimTxTrace(pkHash: string, claims: ClaimBatchItem[]): Promise<string>;
    batchClaim(pkHash: string, claims: ClaimBatchItem[]): Promise<string>;
    getClaimRewardsCallArgs(_jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(_pkHash: string, _jobInfos: string): Promise<string>;
    startSession(pkHash: PublicKey): Promise<string>;
    proveContractCall(pkHash: PublicKey, contractCallArg: ContractCallArgs): Promise<string>;
    proveContractCalls(pkHash: PublicKey, contractCallArgs: ContractCallArgs[]): Promise<string>;
    signAndSubmit(pkHash: PublicKey, signData?: SignData): Promise<string>;
    generateTxTrace(pkHash: PublicKey, callData: ContractCallData): Promise<string>;
    simulateContractCall(pkHash: PublicKey, callData: ContractCallData): Promise<string>;
    proveTxTrace(pkHash: PublicKey, envelopeJson: string): Promise<string>;
    registerUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    addUser(privateKey: PrivateKey, signType: SignType, fingerprint?: string): Promise<PublicKey>;
    getZKPublicKey(privateKey: PrivateKey): Promise<ZKPublicKeyInfo>;
    getRandomKeypair(): Promise<WalletKeyPair>;
    deployContract(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    getDeployContractCmd(deployer: PublicKey, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;
    ping(message: string): Promise<string>;
    getResult(id: QHashOut): Promise<U8Bytes>;
}
export declare class PsyWasmConstantsProvider {
    private static _cache;
    static getAll(): Record<string, any>;
    static get<T = any>(key: keyof typeof WasmConstants | string): T;
    static getRawJson(): string;
    static refresh(): void;
    static get globalUserTreeHeight(): number;
    static get coordinatorUserTreeHeight(): number;
    static get realmUserTreeHeight(): number;
    static get groupRealmHeight(): number;
    static get usersPerRealm(): bigint;
    static get nativeCurrency(): string;
    static get nativeCurrencyName(): string;
    static get nativeCurrencyDecimal(): number;
    static get registerUserFee(): bigint;
    static get deployContractFee(): bigint;
    static get gutaFee(): bigint;
    static get currentNetwork(): string;
    static get configPath(): string;
    static get coordinatorRpcUrl(): string;
    static get realmRpcUrls(): string[];
}
export declare class PsyWasmConfigBuilderProvider {
    static wasmPsyConfigBuilder: WasmPsyConfigBuilder | null;
    static wasmPsyConfig: WasmPsyConfig | null;
    static initBuilder(): WasmPsyConfigBuilder;
    static fromJson(json: string): WasmPsyConfigBuilder;
    static setNetwork(network: string): WasmPsyConfigBuilder;
    static build(): WasmPsyConfig;
    static useNetwork(network: string): void;
    static getCurrentNetwork(): string;
    static listNetworks(): string[];
    static getNetworkJson(network: string): string;
}
//# sourceMappingURL=provider.d.ts.map