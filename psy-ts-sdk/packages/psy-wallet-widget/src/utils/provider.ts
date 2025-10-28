// import {QedRPCProvider, QedUserWalletProvider, QedMemoryTransactionSignerProvider, QedRPCUserProverProvider, IQedUserProverProvider} from "@qstudio/Qed-sdk";
import {
    IQedUserProverProvider,
    MultiCoordinatorRpcProvider,
    MultiRealmRpcProvider,
    QedRPCUserProverProvider,
    QedWasmWebProverProvider,
    RpcConfig,
} from "@qed/psy-sdk";

import { QedUserWalletProvider } from "@qed/psy-sdk/src/wallet/provider";
import { QedMemoryTransactionSignerProvider } from "@qed/psy-sdk/src/zksigner/memory/provider";

function createMemoryWalletProvider(
    globalUserTreeHeight: number,
    realmUserTreeHeight: number,
    coordinatorRpcConfigs: RpcConfig[],
    realmRpcConfigs: RpcConfig[],
    userPerRealm: number,
    proverUrl?: string,
    prove_proxy_url: string[],
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
            global_user_tree_height: globalUserTreeHeight,
            realm_user_tree_height: realmUserTreeHeight,
            users_per_realm: userPerRealm,
            realm_configs: realmRpcConfigs,
            coordinator_configs: coordinatorRpcConfigs,
            prove_proxy_url: prove_proxy_url,
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
