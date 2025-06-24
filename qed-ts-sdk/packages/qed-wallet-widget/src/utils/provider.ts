// import {QedRPCProvider, QedUserWalletProvider, QedMemoryTransactionSignerProvider, QedRPCUserProverProvider, IQedUserProverProvider} from "@qstudio/Qed-sdk";
import { QedMemoryTransactionSignerProvider } from "@qed/qed-sdk/src/zksigner/memory/provider";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";
import { CoordinatorEdgeRpcProvider } from "@qed/qed-sdk";
import { RealmEdgeRpcProvider } from "@qed/qed-sdk";
import { QedRPCUserProverProvider } from "@qed/qed-sdk";

function createMemoryWalletProvider(coordinatorRpcUrl: string, realmRpcUrl: string, proverUrl: string): QedUserWalletProvider {
    const networkId = "regtest";
    const coordinator_rpc = new CoordinatorEdgeRpcProvider(coordinatorRpcUrl);
    const realm_rpc = new RealmEdgeRpcProvider(realmRpcUrl);

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
