// Entry point of a proving thread Worker (see setThreadWorkerFactory in provider.ts). This module
// imports only the wasm-bindgen glue, so a host bundles a small thread worker: one per thread, and
// each one parses its own copy of the script.
import * as proverModule from "./psy_prover";
import { initSync } from "./psy_prover";

/**
 * Waits for the pool's init message, instantiates the prover on the shared memory and joins the
 * pool. Call it synchronously at the top level of the worker script: the init message is posted
 * as soon as the worker exists, and a listener added later would miss it.
 */
export function startProverThreadWorker(): void {
    const scope = globalThis as unknown as {
        addEventListener(type: "message", listener: (event: MessageEvent) => void): void;
        removeEventListener(type: "message", listener: (event: MessageEvent) => void): void;
        postMessage(message: unknown): void;
    };
    const onMessage = (event: MessageEvent) => {
        const data = event.data as {
            type?: string;
            init?: { module_or_path: WebAssembly.Module; memory: WebAssembly.Memory };
            receiver?: number;
        };
        if (data?.type !== "wasm_bindgen_worker_init" || !data.init || data.receiver === undefined) {
            return;
        }
        scope.removeEventListener("message", onMessage);
        try {
            initSync({ module: data.init.module_or_path, memory: data.init.memory });
        } catch (error) {
            scope.postMessage({ type: "wasm_bindgen_worker_error", error: String(error) });
            return;
        }
        scope.postMessage({ type: "wasm_bindgen_worker_ready" });
        (proverModule as unknown as { wbg_rayon_start_worker(receiver: number): void }).wbg_rayon_start_worker(data.receiver);
    };
    scope.addEventListener("message", onMessage);
}
