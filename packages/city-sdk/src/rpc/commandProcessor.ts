import type {
  CityMerkleProof,
  ICityUserState,
  ICityL1Deposit,
  ICityL2BlockState,
  ICityL1Withdrawal,
  TProofValueStoreKV,
} from "./baseTypes";
import { CityRPCProvider } from "./implementation";
import {
  AddWithdrawalRequest,
  CityRPCCommand,
  CityRPCCommandRequest,
  ClaimDepositRequest,
  GetBlockStateRequest,
  GetCityBlockDepositAddressRequest,
  GetCityBlockDepositAddressStringRequest,
  GetCityBlockScriptRequest,
  GetCityRootRequest,
  GetDepositByIdRequest,
  GetDepositByTxidRequest,
  GetDepositHashRequest,
  GetDepositLeafMerkleProofRequest,
  GetDepositTreeRootRequest,
  GetDepositsByIdRequest,
  GetDepositsByTxidRequest,
  GetLatestBlockStateRequest,
  GetProofStoreValueRequest,
  GetProofStoreValuesRequest,
  GetUserByIdRequest,
  GetUserIdsForPublicKeyRequest,
  GetUserMerkleProofByIdRequest,
  GetUserTreeLeafMerkleProofRequest,
  GetUserTreeLeafRequest,
  GetUserTreeRootRequest,
  GetWithdrawalByIdRequest,
  GetWithdrawalHashRequest,
  GetWithdrawalLeafMerkleProofRequest,
  GetWithdrawalTreeRootRequest,
  GetWithdrawalsByIdRequest,
  ICityRPCCommandRequestProcessor,
  ProduceBlockRequest,
  RegisterUserRequest,
  TokenTransferRequest,
} from "./types";

// type TCityRPCRequestHandlers = { [K in CityRPCCommand]: (provider: CityRPCProvider, req: CityRPCCommandRequest & {commandType: K}) => Promise<any> };

const CommandHandler = {
  [CityRPCCommand.GetUserTreeRoot]: (
    provider: CityRPCProvider,
    req: GetUserTreeRootRequest
  ) => {
    return provider.getUserTreeRoot(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetUserIdsForPublicKey]: (
    provider: CityRPCProvider,
    req: GetUserIdsForPublicKeyRequest
  ) => {
    return provider.getUserIdsForPublicKey(req.params.public_key);
  },
  [CityRPCCommand.GetUserById]: (
    provider: CityRPCProvider,
    req: GetUserByIdRequest
  ) => {
    return provider.getUserById(req.params.checkpoint_id, req.params.user_id);
  },
  [CityRPCCommand.GetUserMerkleProofById]: (
    provider: CityRPCProvider,
    req: GetUserMerkleProofByIdRequest
  ) => {
    return provider.getUserMerkleProofById(
      req.params.checkpoint_id,
      req.params.user_id
    );
  },
  [CityRPCCommand.GetUserTreeLeaf]: (
    provider: CityRPCProvider,
    req: GetUserTreeLeafRequest
  ) => {
    return provider.getUserTreeLeaf(
      req.params.checkpoint_id,
      req.params.leaf_id
    );
  },
  [CityRPCCommand.GetUserTreeLeafMerkleProof]: (
    provider: CityRPCProvider,
    req: GetUserTreeLeafMerkleProofRequest
  ) => {
    return provider.getUserTreeLeafMerkleProof(
      req.params.checkpoint_id,
      req.params.leaf_id
    );
  },
  [CityRPCCommand.GetDepositTreeRoot]: (
    provider: CityRPCProvider,
    req: GetDepositTreeRootRequest
  ) => {
    return provider.getDepositTreeRoot(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetDepositById]: (
    provider: CityRPCProvider,
    req: GetDepositByIdRequest
  ) => {
    return provider.getDepositById(
      req.params.checkpoint_id,
      req.params.deposit_id
    );
  },
  [CityRPCCommand.GetDepositsById]: (
    provider: CityRPCProvider,
    req: GetDepositsByIdRequest
  ) => {
    return provider.getDepositsById(
      req.params.checkpoint_id,
      req.params.deposit_ids
    );
  },
  [CityRPCCommand.GetDepositByTxid]: (
    provider: CityRPCProvider,
    req: GetDepositByTxidRequest
  ) => {
    return provider.getDepositByTxid(req.params.transaction_id);
  },
  [CityRPCCommand.GetDepositsByTxid]: (
    provider: CityRPCProvider,
    req: GetDepositsByTxidRequest
  ) => {
    return provider.getDepositsByTxid(req.params.transaction_ids);
  },
  [CityRPCCommand.GetDepositHash]: (
    provider: CityRPCProvider,
    req: GetDepositHashRequest
  ) => {
    return provider.getDepositHash(
      req.params.checkpoint_id,
      req.params.deposit_id
    );
  },
  [CityRPCCommand.GetDepositLeafMerkleProof]: (
    provider: CityRPCProvider,
    req: GetDepositLeafMerkleProofRequest
  ) => {
    return provider.getDepositLeafMerkleProof(
      req.params.checkpoint_id,
      req.params.deposit_id
    );
  },
  [CityRPCCommand.GetBlockState]: (
    provider: CityRPCProvider,
    req: GetBlockStateRequest
  ) => {
    return provider.getBlockState(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetLatestBlockState]: (
    provider: CityRPCProvider,
    req: GetLatestBlockStateRequest
  ) => {
    return provider.getLatestBlockState();
  },
  [CityRPCCommand.GetCityRoot]: (
    provider: CityRPCProvider,
    req: GetCityRootRequest
  ) => {
    return provider.getCityRoot(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetCityBlockScript]: (
    provider: CityRPCProvider,
    req: GetCityBlockScriptRequest
  ) => {
    return provider.getCityBlockScript(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetCityBlockDepositAddress]: (
    provider: CityRPCProvider,
    req: GetCityBlockDepositAddressRequest
  ) => {
    return provider.getCityBlockDepositAddress(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetCityBlockDepositAddressString]: (
    provider: CityRPCProvider,
    req: GetCityBlockDepositAddressStringRequest
  ) => {
    return provider.getCityBlockDepositAddressString(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetWithdrawalTreeRoot]: (
    provider: CityRPCProvider,
    req: GetWithdrawalTreeRootRequest
  ) => {
    return provider.getWithdrawalTreeRoot(req.params.checkpoint_id);
  },
  [CityRPCCommand.GetWithdrawalById]: (
    provider: CityRPCProvider,
    req: GetWithdrawalByIdRequest
  ) => {
    return provider.getWithdrawalById(
      req.params.checkpoint_id,
      req.params.withdrawal_id
    );
  },
  [CityRPCCommand.GetWithdrawalsById]: (
    provider: CityRPCProvider,
    req: GetWithdrawalsByIdRequest
  ) => {
    return provider.getWithdrawalsById(
      req.params.checkpoint_id,
      req.params.withdrawal_ids
    );
  },
  [CityRPCCommand.GetWithdrawalHash]: (
    provider: CityRPCProvider,
    req: GetWithdrawalHashRequest
  ) => {
    return provider.getWithdrawalHash(
      req.params.checkpoint_id,
      req.params.withdrawal_id
    );
  },
  [CityRPCCommand.GetWithdrawalLeafMerkleProof]: (
    provider: CityRPCProvider,
    req: GetWithdrawalLeafMerkleProofRequest
  ) => {
    return provider.getWithdrawalLeafMerkleProof(
      req.params.checkpoint_id,
      req.params.withdrawal_id
    );
  },
  [CityRPCCommand.GetProofStoreValue]: (
    provider: CityRPCProvider,
    req: GetProofStoreValueRequest
  ) => {
    return provider.getProofStoreValue(req.params.key);
  },
  [CityRPCCommand.GetProofStoreValues]: (
    provider: CityRPCProvider,
    req: GetProofStoreValuesRequest
  ) => {
    return provider.getProofStoreValues(req.params.keys);
  },
  [CityRPCCommand.RegisterUser]: (
    provider: CityRPCProvider,
    req: RegisterUserRequest
  ) => {
    return provider.registerUser(req.params);
  },
  [CityRPCCommand.AddWithdrawal]: (
    provider: CityRPCProvider,
    req: AddWithdrawalRequest
  ) => {
    return provider.addWithdrawal(req.params);
  },
  [CityRPCCommand.ClaimDeposit]: (
    provider: CityRPCProvider,
    req: ClaimDepositRequest
  ) => {
    return provider.claimDeposit(req.params);
  },
  [CityRPCCommand.TokenTransfer]: (
    provider: CityRPCProvider,
    req: TokenTransferRequest
  ) => {
    return provider.tokenTransfer(req.params);
  },
  [CityRPCCommand.ProduceBlock]: (
    provider: CityRPCProvider,
    _: ProduceBlockRequest
  ) => {
    return provider.produceBlock();
  },
};
class CityRPCCommandProcessor implements ICityRPCCommandRequestProcessor {
  rpcProvider: CityRPCProvider;
  constructor(rpcProvider: CityRPCProvider) {
    this.rpcProvider = rpcProvider;
  }
  processRequest(request: GetUserTreeRootRequest): Promise<string>;
  processRequest(request: GetUserIdsForPublicKeyRequest): Promise<number[]>;
  processRequest(request: GetUserByIdRequest): Promise<ICityUserState>;
  processRequest(
    request: GetUserMerkleProofByIdRequest
  ): Promise<CityMerkleProof>;
  processRequest(request: GetUserTreeLeafRequest): Promise<string>;
  processRequest(
    request: GetUserTreeLeafMerkleProofRequest
  ): Promise<CityMerkleProof>;
  processRequest(request: GetDepositTreeRootRequest): Promise<string>;
  processRequest(request: GetDepositByIdRequest): Promise<ICityL1Deposit>;
  processRequest(request: GetDepositsByIdRequest): Promise<ICityL1Deposit[]>;
  processRequest(request: GetDepositByTxidRequest): Promise<ICityL1Deposit>;
  processRequest(request: GetDepositsByTxidRequest): Promise<ICityL1Deposit[]>;
  processRequest(request: GetDepositHashRequest): Promise<string>;
  processRequest(
    request: GetDepositLeafMerkleProofRequest
  ): Promise<CityMerkleProof>;
  processRequest(request: GetBlockStateRequest): Promise<ICityL2BlockState>;
  processRequest(
    request: GetLatestBlockStateRequest
  ): Promise<ICityL2BlockState>;
  processRequest(request: GetCityRootRequest): Promise<string>;
  processRequest(request: GetCityBlockScriptRequest): Promise<string>;
  processRequest(request: GetCityBlockDepositAddressRequest): Promise<string>;
  processRequest(
    request: GetCityBlockDepositAddressStringRequest
  ): Promise<string>;
  processRequest(request: GetWithdrawalTreeRootRequest): Promise<string>;
  processRequest(request: GetWithdrawalByIdRequest): Promise<ICityL1Withdrawal>;
  processRequest(
    request: GetWithdrawalsByIdRequest
  ): Promise<ICityL1Withdrawal[]>;
  processRequest(request: GetWithdrawalHashRequest): Promise<string>;
  processRequest(
    request: GetWithdrawalLeafMerkleProofRequest
  ): Promise<CityMerkleProof>;
  processRequest(
    request: GetProofStoreValueRequest
  ): Promise<TProofValueStoreKV>;
  processRequest(
    request: GetProofStoreValuesRequest
  ): Promise<TProofValueStoreKV[]>;
  processRequest(request: RegisterUserRequest): Promise<void>;
  processRequest(request: AddWithdrawalRequest): Promise<void>;
  processRequest(request: ClaimDepositRequest): Promise<void>;
  processRequest(request: TokenTransferRequest): Promise<void>;
  processRequest(request: ProduceBlockRequest): Promise<void>;
  async processRequest(request: CityRPCCommandRequest) {
    if (!Object.hasOwnProperty.call(CommandHandler, request.commandType)) {
      throw new Error(
        `CityRPCCommandProcessor: Unknown command type: ${request.commandType}`
      );
    }
    return CommandHandler[request.commandType](
      this.rpcProvider,
      request as any
    );
  }
}

export { CityRPCCommandProcessor };
