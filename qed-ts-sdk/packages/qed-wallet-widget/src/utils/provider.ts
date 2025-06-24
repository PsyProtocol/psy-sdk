// import {QedRPCProvider, QedUserWalletProvider, QedMemoryTransactionSignerProvider, QedRPCUserProverProvider, IQedUserProverProvider} from "@qstudio/Qed-sdk";
import { QedMemoryTransactionSignerProvider } from "@qed/qed-sdk/src/zksigner/memory/provider";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";
import { MultiCoordinatorRpcProvider, MultiRealmRpcProvider, RpcConfig } from "@qed/qed-sdk";
import { QedRPCUserProverProvider } from "@qed/qed-sdk";

function createMemoryWalletProvider(
    coordinatorRpcConfigs: RpcConfig[],
    realmRpcConfigs: RpcConfig[],
    userPerRealm: number,
    proverUrl: string
): QedUserWalletProvider {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(coordinatorRpcConfigs);
    const realm_rpc = new MultiRealmRpcProvider(realmRpcConfigs, userPerRealm);

    const userProver = new QedRPCUserProverProvider(proverUrl);

    const transactionSignerProvider = new QedMemoryTransactionSignerProvider(userProver, networkId);

    const walletProvider = new QedUserWalletProvider(
        networkId,
        coordinator_rpc,
        realm_rpc,
        transactionSignerProvider,
        userProver
    );
    return walletProvider;
}

export { createMemoryWalletProvider };
