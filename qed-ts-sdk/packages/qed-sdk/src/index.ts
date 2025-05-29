export type { ISimpleHTTPRequest, ISimpleHTTPResponse, IHTTPClient } from "./http/types";
export { FetchHTTPClient } from "./http/fetchClient";
export { CityRPCCommand } from "./rpc/types";
export type {
    ICityRPCProvider,
    ICityRPCCommandRequestProcessor,
    CityRPCCommandRequest,
    GetUserTreeRootRequest,
    GetUserIdsForPublicKeyRequest,
    GetUserByIdRequest,
    GetUserMerkleProofByIdRequest,
    GetUserTreeLeafRequest,
    GetUserTreeLeafMerkleProofRequest,
    GetDepositTreeRootRequest,
    GetDepositByIdRequest,
    GetDepositsByIdRequest,
    GetDepositByTxidRequest,
    GetDepositsByTxidRequest,
    GetDepositHashRequest,
    GetDepositLeafMerkleProofRequest,
    GetBlockStateRequest,
    GetLatestBlockStateRequest,
    GetCityRootRequest,
    GetCityBlockScriptRequest,
    GetCityBlockDepositAddressRequest,
    GetCityBlockDepositAddressStringRequest,
    GetWithdrawalTreeRootRequest,
    GetWithdrawalByIdRequest,
    GetWithdrawalsByIdRequest,
    GetWithdrawalHashRequest,
    GetWithdrawalLeafMerkleProofRequest,
    GetProofStoreValueRequest,
    GetProofStoreValuesRequest,
    RegisterUserRequest,
    AddWithdrawalRequest,
    ClaimDepositRequest,
    TokenTransferRequest,
    ProduceBlockRequest,
} from "./rpc/types";

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

export { CityRPCProvider } from "./rpc/implementation";
export { CityRPCProviderWithCache } from "./rpc/cached";

export { CityRPCCommandProcessor } from "./rpc/commandProcessor";

export type {
    ICoreCityUserInfo,
    ICityCompleteUserInfo,
    ICityUserWallet,
    ICityUserWalletProvider,
} from "./wallet/types";
export * from "./wallet/index";

export type {
    ICityTransactionSigner,
    ICityTransactionSignerProvider,
    TCityTransactionSignerAbility,
    TCityTransactionSignerProviderAbility,
} from "./zksigner/types";

export * from "./zksigner/memory";
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
