import { DogeNetworkId } from "doge-sdk/dist/types";
import type {
    CityHash,
    Hash256,
    Hash160,
    QProvingJobDataIDSerializedWrapped,
    CityMerkleProof,
    ICityUserState,
    ICityL1Deposit,
    ICityL2BlockState,
    ICityL1Withdrawal,
    ICityRegisterUserRPCRequest,
    ICityAddWithdrawalRPCRequest,
    ICityClaimDepositRPCRequest,
    ICityTokenTransferRPCRequest,
    TProofValueStoreKV,
    ISimpleKVPair,
} from "./baseTypes";

interface ICityRPCProvider {
    getUserTreeRoot(checkpoint_id: number): Promise<CityHash>;
    getUserIdsForPublicKey(public_key: CityHash): Promise<number[]>;
    getUserById(checkpoint_id: number, user_id: number): Promise<ICityUserState>;
    getUserMerkleProofById(checkpoint_id: number, user_id: number): Promise<CityMerkleProof>;
    getUserTreeLeaf(checkpoint_id: number, leaf_id: number): Promise<CityHash>;
    getUserTreeLeafMerkleProof(checkpoint_id: number, leaf_id: number): Promise<CityMerkleProof>;
    getDepositTreeRoot(checkpoint_id: number): Promise<CityHash>;
    getDepositById(checkpoint_id: number, deposit_id: number): Promise<ICityL1Deposit>;
    getDepositsById(checkpoint_id: number, deposit_ids: number[]): Promise<ICityL1Deposit[]>;
    getDepositByTxid(transaction_id: Hash256): Promise<ICityL1Deposit>;
    getDepositsByTxid(transaction_ids: Hash256[]): Promise<ICityL1Deposit[]>;
    getDepositHash(checkpoint_id: number, deposit_id: number): Promise<CityHash>;
    getDepositLeafMerkleProof(checkpoint_id: number, deposit_id: number): Promise<CityMerkleProof>;
    getBlockState(checkpoint_id: number): Promise<ICityL2BlockState>;
    getLatestBlockState(): Promise<ICityL2BlockState>;
    getCityRoot(checkpoint_id: number): Promise<CityHash>;
    getCityBlockScript(checkpoint_id: number): Promise<string>;
    getCityBlockDepositAddress(checkpoint_id: number): Promise<Hash160>;
    getCityBlockDepositAddressString(checkpoint_id: number): Promise<string>;
    getWithdrawalTreeRoot(checkpoint_id: number): Promise<CityHash>;
    getWithdrawalById(checkpoint_id: number, withdrawal_id: number): Promise<ICityL1Withdrawal>;
    getWithdrawalsById(checkpoint_id: number, withdrawal_ids: number[]): Promise<ICityL1Withdrawal[]>;
    getWithdrawalHash(checkpoint_id: number, withdrawal_id: number): Promise<CityHash>;
    getWithdrawalLeafMerkleProof(checkpoint_id: number, withdrawal_id: number): Promise<CityMerkleProof>;
    getProofStoreValue(key: QProvingJobDataIDSerializedWrapped): Promise<string>;
    getProofStoreValues(keys: QProvingJobDataIDSerializedWrapped[]): Promise<TProofValueStoreKV[]>;
    getProofStoreJobWitness(key: QProvingJobDataIDSerializedWrapped): Promise<any>;
    getProofStoreJobWitnesses(key: QProvingJobDataIDSerializedWrapped[]): Promise<ISimpleKVPair<string, any>[]>;
    registerUser<F>(req: ICityRegisterUserRPCRequest): Promise<void>;
    addWithdrawal(req: ICityAddWithdrawalRPCRequest): Promise<void>;
    claimDeposit(req: ICityClaimDepositRPCRequest): Promise<void>;
    tokenTransfer(req: ICityTokenTransferRPCRequest): Promise<void>;
    produceBlock(): Promise<void>;
    getNetworkMagic(): string;
    getDogeNetworkId(): DogeNetworkId;
}
enum CityRPCCommand {
    GetUserTreeRoot = "cr_getUserTreeRoot",
    GetUserIdsForPublicKey = "cr_getUserIdsForPublicKey",
    GetUserById = "cr_getUserById",
    GetUserMerkleProofById = "cr_getUserMerkleProofById",
    GetUserTreeLeaf = "cr_getUserTreeLeaf",
    GetUserTreeLeafMerkleProof = "cr_getUserTreeLeafMerkleProof",
    GetDepositTreeRoot = "cr_getDepositTreeRoot",
    GetDepositById = "cr_getDepositById",
    GetDepositsById = "cr_getDepositsById",
    GetDepositByTxid = "cr_getDepositByTxid",
    GetDepositsByTxid = "cr_getDepositsByTxid",
    GetDepositHash = "cr_getDepositHash",
    GetDepositLeafMerkleProof = "cr_getDepositLeafMerkleProof",
    GetBlockState = "cr_getBlockState",
    GetLatestBlockState = "cr_getLatestBlockState",
    GetCityRoot = "cr_getCityRoot",
    GetCityBlockScript = "cr_getCityBlockScript",
    GetCityBlockDepositAddress = "cr_getCityBlockDepositAddress",
    GetCityBlockDepositAddressString = "cr_getCityBlockDepositAddressString",
    GetWithdrawalTreeRoot = "cr_getWithdrawalTreeRoot",
    GetWithdrawalById = "cr_getWithdrawalById",
    GetWithdrawalsById = "cr_getWithdrawalsById",
    GetWithdrawalHash = "cr_getWithdrawalHash",
    GetWithdrawalLeafMerkleProof = "cr_getWithdrawalLeafMerkleProof",
    GetProofStoreValue = "cr_getProofStoreValue",
    GetProofStoreValues = "cr_getProofStoreValues",
    RegisterUser = "cr_register_user",
    AddWithdrawal = "cr_add_withdrawal",
    ClaimDeposit = "cr_claim_deposit",
    TokenTransfer = "cr_token_transfer",
    ProduceBlock = "cr_produce_block",
}

interface ICityRPCCommandRequestBase<T> {
    commandType: CityRPCCommand;
    params: T;
}

interface ICityRPCCommandRequest<C extends CityRPCCommand, T> extends ICityRPCCommandRequestBase<T> {
    commandType: C;
    params: T;
}

interface ICheckpointIdRequest {
    checkpoint_id: number;
}
interface ICheckpointIdAndUserIdRequest {
    checkpoint_id: number;
    user_id: number;
}
interface ICheckpointIdAndLeafIdRequest {
    checkpoint_id: number;
    leaf_id: number;
}
interface ICheckpointIdAndDepositIdRequest {
    checkpoint_id: number;
    deposit_id: number;
}
interface ICheckpointIdAndDepositIdsRequest {
    checkpoint_id: number;
    deposit_ids: number[];
}
interface ICheckpointIdAndTransactionIdRequest {
    checkpoint_id: number;
    transaction_id: Hash256;
}
interface ITransactionIdRequest {
    transaction_id: Hash256;
}
interface ICheckpointIdAndTransactionIdsRequest {
    checkpoint_id: number;
    transaction_ids: Hash256[];
}
interface ICheckpointIdAndWithdrawalIdRequest {
    checkpoint_id: number;
    withdrawal_id: number;
}
interface ICheckpointIdAndWithdrawalIdsRequest {
    checkpoint_id: number;
    withdrawal_ids: number[];
}
interface ICheckpointIdAndProofStoreKeyRequest {
    key: QProvingJobDataIDSerializedWrapped;
}
interface ICheckpointIdAndProofStoreKeysRequest {
    keys: QProvingJobDataIDSerializedWrapped[];
}
type GetUserTreeRootRequest = ICityRPCCommandRequest<CityRPCCommand.GetUserTreeRoot, ICheckpointIdRequest>;
type GetUserIdsForPublicKeyRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetUserIdsForPublicKey,
    { public_key: CityHash }
>;
type GetUserByIdRequest = ICityRPCCommandRequest<CityRPCCommand.GetUserById, ICheckpointIdAndUserIdRequest>;
type GetUserMerkleProofByIdRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetUserMerkleProofById,
    ICheckpointIdAndUserIdRequest
>;
type GetUserTreeLeafRequest = ICityRPCCommandRequest<CityRPCCommand.GetUserTreeLeaf, ICheckpointIdAndLeafIdRequest>;
type GetUserTreeLeafMerkleProofRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetUserTreeLeafMerkleProof,
    ICheckpointIdAndLeafIdRequest
>;
type GetDepositTreeRootRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositTreeRoot, ICheckpointIdRequest>;
type GetDepositByIdRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositById, ICheckpointIdAndDepositIdRequest>;
type GetDepositsByIdRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositsById, ICheckpointIdAndDepositIdsRequest>;
type GetDepositByTxidRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositByTxid, { transaction_id: string }>;
type GetDepositsByTxidRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositsByTxid, { transaction_ids: string[] }>;
type GetDepositHashRequest = ICityRPCCommandRequest<CityRPCCommand.GetDepositHash, ICheckpointIdAndDepositIdRequest>;
type GetDepositLeafMerkleProofRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetDepositLeafMerkleProof,
    ICheckpointIdAndDepositIdRequest
>;
type GetBlockStateRequest = ICityRPCCommandRequest<CityRPCCommand.GetBlockState, ICheckpointIdRequest>;
type GetLatestBlockStateRequest = ICityRPCCommandRequest<CityRPCCommand.GetLatestBlockState, undefined>;
type GetCityRootRequest = ICityRPCCommandRequest<CityRPCCommand.GetCityRoot, ICheckpointIdRequest>;
type GetCityBlockScriptRequest = ICityRPCCommandRequest<CityRPCCommand.GetCityBlockScript, ICheckpointIdRequest>;
type GetCityBlockDepositAddressRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetCityBlockDepositAddress,
    ICheckpointIdRequest
>;
type GetCityBlockDepositAddressStringRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetCityBlockDepositAddressString,
    ICheckpointIdRequest
>;
type GetWithdrawalTreeRootRequest = ICityRPCCommandRequest<CityRPCCommand.GetWithdrawalTreeRoot, ICheckpointIdRequest>;
type GetWithdrawalByIdRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetWithdrawalById,
    ICheckpointIdAndWithdrawalIdRequest
>;
type GetWithdrawalsByIdRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetWithdrawalsById,
    ICheckpointIdAndWithdrawalIdsRequest
>;
type GetWithdrawalHashRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetWithdrawalHash,
    ICheckpointIdAndWithdrawalIdRequest
>;
type GetWithdrawalLeafMerkleProofRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetWithdrawalLeafMerkleProof,
    ICheckpointIdAndWithdrawalIdRequest
>;
type GetProofStoreValueRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetProofStoreValue,
    ICheckpointIdAndProofStoreKeyRequest
>;
type GetProofStoreValuesRequest = ICityRPCCommandRequest<
    CityRPCCommand.GetProofStoreValues,
    ICheckpointIdAndProofStoreKeysRequest
>;
type RegisterUserRequest = ICityRPCCommandRequest<CityRPCCommand.RegisterUser, ICityRegisterUserRPCRequest>;
type AddWithdrawalRequest = ICityRPCCommandRequest<CityRPCCommand.AddWithdrawal, ICityAddWithdrawalRPCRequest>;
type ClaimDepositRequest = ICityRPCCommandRequest<CityRPCCommand.ClaimDeposit, ICityClaimDepositRPCRequest>;
type TokenTransferRequest = ICityRPCCommandRequest<CityRPCCommand.TokenTransfer, ICityTokenTransferRPCRequest>;
type ProduceBlockRequest = ICityRPCCommandRequest<CityRPCCommand.ProduceBlock, undefined>;

type CityRPCCommandRequest =
    | GetUserTreeRootRequest
    | GetUserIdsForPublicKeyRequest
    | GetUserByIdRequest
    | GetUserMerkleProofByIdRequest
    | GetUserTreeLeafRequest
    | GetUserTreeLeafMerkleProofRequest
    | GetDepositTreeRootRequest
    | GetDepositByIdRequest
    | GetDepositsByIdRequest
    | GetDepositByTxidRequest
    | GetDepositsByTxidRequest
    | GetDepositHashRequest
    | GetDepositLeafMerkleProofRequest
    | GetBlockStateRequest
    | GetLatestBlockStateRequest
    | GetCityRootRequest
    | GetCityBlockScriptRequest
    | GetCityBlockDepositAddressRequest
    | GetCityBlockDepositAddressStringRequest
    | GetWithdrawalTreeRootRequest
    | GetWithdrawalByIdRequest
    | GetWithdrawalsByIdRequest
    | GetWithdrawalHashRequest
    | GetWithdrawalLeafMerkleProofRequest
    | GetProofStoreValueRequest
    | GetProofStoreValuesRequest
    | RegisterUserRequest
    | AddWithdrawalRequest
    | ClaimDepositRequest
    | TokenTransferRequest
    | ProduceBlockRequest;

interface ICityRPCCommandRequestProcessor {
    processRequest(request: GetUserTreeRootRequest): Promise<CityHash>;
    processRequest(request: GetUserIdsForPublicKeyRequest): Promise<number[]>;
    processRequest(request: GetUserByIdRequest): Promise<ICityUserState>;
    processRequest(request: GetUserMerkleProofByIdRequest): Promise<CityMerkleProof>;
    processRequest(request: GetUserTreeLeafRequest): Promise<CityHash>;
    processRequest(request: GetUserTreeLeafMerkleProofRequest): Promise<CityMerkleProof>;
    processRequest(request: GetDepositTreeRootRequest): Promise<CityHash>;
    processRequest(request: GetDepositByIdRequest): Promise<ICityL1Deposit>;
    processRequest(request: GetDepositsByIdRequest): Promise<ICityL1Deposit[]>;
    processRequest(request: GetDepositByTxidRequest): Promise<ICityL1Deposit>;
    processRequest(request: GetDepositsByTxidRequest): Promise<ICityL1Deposit[]>;
    processRequest(request: GetDepositHashRequest): Promise<CityHash>;
    processRequest(request: GetDepositLeafMerkleProofRequest): Promise<CityMerkleProof>;
    processRequest(request: GetBlockStateRequest): Promise<ICityL2BlockState>;
    processRequest(request: GetLatestBlockStateRequest): Promise<ICityL2BlockState>;
    processRequest(request: GetCityRootRequest): Promise<CityHash>;
    processRequest(request: GetCityBlockScriptRequest): Promise<string>;
    processRequest(request: GetCityBlockDepositAddressRequest): Promise<Hash160>;
    processRequest(request: GetCityBlockDepositAddressStringRequest): Promise<string>;
    processRequest(request: GetWithdrawalTreeRootRequest): Promise<CityHash>;
    processRequest(request: GetWithdrawalByIdRequest): Promise<ICityL1Withdrawal>;
    processRequest(request: GetWithdrawalsByIdRequest): Promise<ICityL1Withdrawal[]>;
    processRequest(request: GetWithdrawalHashRequest): Promise<CityHash>;
    processRequest(request: GetWithdrawalLeafMerkleProofRequest): Promise<CityMerkleProof>;
    processRequest(request: GetProofStoreValueRequest): Promise<TProofValueStoreKV>;
    processRequest(request: GetProofStoreValuesRequest): Promise<TProofValueStoreKV[]>;
    processRequest(request: RegisterUserRequest): Promise<void>;
    processRequest(request: AddWithdrawalRequest): Promise<void>;
    processRequest(request: ClaimDepositRequest): Promise<void>;
    processRequest(request: TokenTransferRequest): Promise<void>;
    processRequest(request: ProduceBlockRequest): Promise<void>;
}

export { CityRPCCommand };
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
};
