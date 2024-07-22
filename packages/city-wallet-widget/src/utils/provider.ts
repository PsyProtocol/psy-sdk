import {CityRPCProvider, CityUserWalletProvider, CityMemoryTransactionSignerProvider, CityRPCUserProverProvider, ICityUserProverProvider} from "@qstudio/city-sdk";

function createMemoryWalletProvider(rpcUrl: string, proverUrl: string): CityUserWalletProvider{
  const rpc = new CityRPCProvider(rpcUrl);
  const userProver = new CityRPCUserProverProvider(proverUrl);

  const transactionSignerProvider = new CityMemoryTransactionSignerProvider(userProver, rpc.networkId);

  const walletProvider = new CityUserWalletProvider(rpc.networkId, rpc, transactionSignerProvider, userProver);
  return walletProvider;
}

export {
  createMemoryWalletProvider,
}