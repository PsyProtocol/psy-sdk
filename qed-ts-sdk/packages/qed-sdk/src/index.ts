export type { ISimpleHTTPRequest, ISimpleHTTPResponse, IHTTPClient } from "./http";
export { FetchHTTPClient } from "./http";
export type {
    QedHash,
    Hash256,
    SCFelt,
    Hash160,
    CompressedPublicKeyHex,
    QProvingJobDataIDSerializedWrapped,
    QedMerkleProof,
    QedDeltaMerkleProof,
    ISimpleKVPair,
} from "./core";

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
