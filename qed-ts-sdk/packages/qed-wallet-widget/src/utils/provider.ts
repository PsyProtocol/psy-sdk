// import {QedRPCProvider, QedUserWalletProvider, QedMemoryTransactionSignerProvider, QedRPCUserProverProvider, IQedUserProverProvider} from "@qstudio/Qed-sdk";
import {
    IQedUserProverProvider,
    MultiCoordinatorRpcProvider,
    MultiRealmRpcProvider,
    QedRPCUserProverProvider,
    QedWasmWebProverProvider,
    RpcConfig,
} from "@qed/qed-sdk";

// Import WASM binary data synchronously
import { initSync } from "@qed/qed-sdk/src/local-web-prover";
import wasmUrl from "@qed/qed-sdk/src/local-web-prover/qed_user_prover_bg.wasm?url";
import { wasmBinary } from "@qed/qed-sdk/src/local-web-prover/wasm-binary";
import { QedUserWalletProvider } from "@qed/qed-sdk/src/wallet/provider";
import { QedMemoryTransactionSignerProvider } from "@qed/qed-sdk/src/zksigner/memory/provider";


// Synchronous WASM initialization function
function initWasmSync(): void {
    try {
        // Fetch WASM binary synchronously without setting responseType
        const xhr = new XMLHttpRequest();
        xhr.open('GET', wasmUrl, false); // false = synchronous
        // Don't set responseType for synchronous requests
        xhr.overrideMimeType('text/plain; charset=x-user-defined');
        xhr.send();
        
        if (xhr.status !== 200) {
            throw new Error(`Failed to fetch WASM: ${xhr.status}`);
        }
        
        // Convert response text to Uint8Array
        const responseText = xhr.responseText;
        const wasmBinary = new Uint8Array(responseText.length);
        for (let i = 0; i < responseText.length; i++) {
            wasmBinary[i] = responseText.charCodeAt(i) & 0xff;
        }
        
        // Initialize synchronously with binary data
        initSync(wasmBinary);
        
        console.log('WASM initialized synchronously');
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        throw error;
    }
}


// Synchronous WASM initialization function
function initStaticWasmSync(): void {
    try {
        // Initialize synchronously with pre-compiled binary data
        initSync(wasmBinary);
        
        console.log('WASM initialized synchronously from binary data');
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        throw error;
    }
}

function createMemoryWalletProvider(
    coordinatorRpcConfigs: RpcConfig[],
    realmRpcConfigs: RpcConfig[],
    userPerRealm: number,
    proverUrl: string
): QedUserWalletProvider {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(coordinatorRpcConfigs);
    const realm_rpc = new MultiRealmRpcProvider(realmRpcConfigs, userPerRealm);
    let userProver: IQedUserProverProvider | undefined;
    if (proverUrl != null && proverUrl.length > 0) {
        userProver = new QedRPCUserProverProvider(proverUrl);
    } else {
        // Synchronously initialize WASM before creating provider
        // initWasmSync();
        // initStaticWasmSync();
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
