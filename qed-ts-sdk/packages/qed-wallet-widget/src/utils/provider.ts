// import {QedRPCProvider, QedUserWalletProvider, QedMemoryTransactionSignerProvider, QedRPCUserProverProvider, IQedUserProverProvider} from "@qstudio/Qed-sdk";
import {
    IQedUserProverProvider,
    MultiCoordinatorRpcProvider,
    MultiRealmRpcProvider,
    QedRPCUserProverProvider,
    QedWasmWebProverProvider,
    RpcConfig,
} from "@qed/qed-sdk";

import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";
import { QedMemoryTransactionSignerProvider } from "@qed/qed-sdk/src/zksigner/memory/provider";

function createMemoryWalletProvider(
    coordinatorRpcConfigs: RpcConfig[],
    realmRpcConfigs: RpcConfig[],
    userPerRealm: number,
    proverUrl?: string
): QedUserWalletProvider {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(coordinatorRpcConfigs);
    const realm_rpc = new MultiRealmRpcProvider(realmRpcConfigs, userPerRealm);
    let userProver: IQedUserProverProvider | undefined;
    if (proverUrl != null && proverUrl.length > 0) {
        userProver = new QedRPCUserProverProvider(proverUrl);
    } else {
        // Synchronously initialize WASM before creating provider
        userProver = new QedWasmWebProverProvider({
            users_per_realm: userPerRealm,
            realm_configs: realmRpcConfigs,
            coordinator_configs: coordinatorRpcConfigs,
        });
    }

    const transactionSignerProvider = new QedMemoryTransactionSignerProvider(userProver, networkId);

    return new QedUserWalletProvider(
        networkId,
        coordinator_rpc,
        realm_rpc,
        transactionSignerProvider,
        userProver
    );
}

export { createMemoryWalletProvider };
