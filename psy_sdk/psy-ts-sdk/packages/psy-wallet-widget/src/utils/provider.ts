// import {PsyRPCProvider, PsyUserWalletProvider, PsyMemoryTransactionSignerProvider, PsyRPCUserProverProvider, IPsyUserProverProvider} from "@qstudio/Psy-sdk";
import {
    IPsyUserProverProvider,
    MultiCoordinatorRpcProvider,
    MultiRealmRpcProvider,
    PsyRPCUserProverProvider,
    PsyWasmWebProverProvider,
    RpcConfig,
} from "@psy/psy-sdk";

import { PsyUserWalletProvider } from "@psy/psy-sdk/src/wallet/provider";
import { PsyMemoryTransactionSignerProvider } from "@psy/psy-sdk/src/zksigner/memory/provider";

function createMemoryWalletProvider(
    globalUserTreeHeight: number,
    realmUserTreeHeight: number,
    coordinatorRpcConfigs: RpcConfig[],
    realmRpcConfigs: RpcConfig[],
    userPerRealm: number,
    proverUrl?: string,
    prove_proxy_url: string[],
): PsyUserWalletProvider {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(coordinatorRpcConfigs);
    const realm_rpc = new MultiRealmRpcProvider(realmRpcConfigs, userPerRealm);
    let userProver: IPsyUserProverProvider | undefined;
    if (proverUrl != null && proverUrl.length > 0) {
        userProver = new PsyRPCUserProverProvider(proverUrl);
    } else {
        // Synchronously initialize WASM before creating provider
        userProver = new PsyWasmWebProverProvider({
            global_user_tree_height: globalUserTreeHeight,
            realm_user_tree_height: realmUserTreeHeight,
            users_per_realm: userPerRealm,
            realm_configs: realmRpcConfigs,
            coordinator_configs: coordinatorRpcConfigs,
            prove_proxy_url: prove_proxy_url,
        });
    }

    const transactionSignerProvider = new PsyMemoryTransactionSignerProvider(userProver, networkId);

    return new PsyUserWalletProvider(
        networkId,
        coordinator_rpc,
        realm_rpc,
        transactionSignerProvider,
        userProver
    );
}

export { createMemoryWalletProvider };
