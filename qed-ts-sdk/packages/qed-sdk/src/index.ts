export type { ISimpleHTTPRequest, ISimpleHTTPResponse, IHTTPClient } from "./http/types";
export { FetchHTTPClient } from "./http/fetchClient";
export type {
    CityHash,
    Hash256,
    SCFelt,
    Hash160,
    CompressedPublicKeyHex,
    QProvingJobDataIDSerializedWrapped,
    CityMerkleProof,
    CityDeltaMerkleProof,
    ICityUserState,
    ICityL1Deposit,
    ICityL2BlockState,
    ICityL1Withdrawal,
    ISimpleKVPair,
    ICityRegisterUserRPCRequest,
    ICityAddWithdrawalRPCRequest,
    ICityClaimDepositRPCRequest,
    ICityTokenTransferRPCRequest,
    TProofValueStoreKV,
} from "./rpc/baseTypes";

export * from "./utils/felt";

// Enhanced RPC Provider exports
export {
    Provider,
    type CacheConfig,
    type RetryConfig,
    type MultiProviderConfig,
    type ClientConfig,
    type ProviderHealth,
} from "./provider";

// Coordinator Edge RPC exports
export { CoordinatorEdgeRpcProvider, ICoordinatorEdgeRpcProvider } from "./coord-edge-rpc";

// Realm Edge RPC exports
export { RealmEdgeRpcProvider, IRealmEdgeRpcProvider } from "./realm-edge-rpc";

// QED User Prover RPC exports
export { QEDRPCUserProverProvider, IQEDUserProverProvider } from "./local-prover-rpc";
