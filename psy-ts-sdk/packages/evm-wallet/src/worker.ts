/**
 * @psy-protocol/evm-wallet/worker — the prover Web Worker module.
 *
 * Ported VERBATIM from the mode-a app's unified/prover-worker.ts (only the
 * wasm-codec import path changed). Ships as its own side-effectful ESM
 * entrypoint so the CONSUMER's bundler resolves the worker URL: the canonical
 * integration is a 1-line app file `import '@psy-protocol/evm-wallet/worker'`
 * passed to createPsyWallet via `createWorker`. The engine's dist fallback
 * (new Worker(new URL('./worker.mjs', import.meta.url))) targets this same
 * built module.
 *
 * WHY THIS EXISTS: the SDK's PsyWasmWebProverProvider runs multi-second Plonky2
 * proofs synchronously inside wasm-bindgen futures; on the page's main thread
 * that freezes the UI for the whole proof. This worker owns the WASM prover so
 * the main thread stays free ("proving…" stays smooth). The static
 * PsyWasmWebProverProvider.wasmServer is shared per worker thread, and every
 * call routes through ONE worker, so the signer registerUser installs is the
 * SAME instance a later execContractCall(pkHash, …) signs with.
 */
import { PsyJSON, PsyWasmWebProverProvider } from "@psy-protocol/psy-sdk";
import {
  deserializeWasmValue,
  serializeWasmValue,
  type SerializedWasmValue,
  type WasmWorkerRequest,
} from "./prover/wasm-codec";

console.log("[psy-diag] prover-worker module loaded");

let cachedProvider: PsyWasmWebProverProvider | null = null;
let cachedConfigJson: string | null = null;
let wasmServerReady: Promise<void> | null = null;

async function ensureProviderReady(configJson: string): Promise<PsyWasmWebProverProvider> {
  if (cachedProvider && cachedConfigJson === configJson && wasmServerReady) {
    await wasmServerReady;
    return cachedProvider;
  }
  // SDK 2.0.4+ exposes ensureWasmServer / runWasmServerCall. Initialize the
  // singleton WasmRpcServer explicitly and await it before constructing the
  // provider, then the provider's instance methods (which also route through
  // runWasmServerCall) run against the resolved server.
  wasmServerReady = (async () => {
    await PsyWasmWebProverProvider.ensureWasmServer(configJson);
  })();
  await wasmServerReady;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  cachedProvider = new PsyWasmWebProverProvider(PsyJSON.parse(configJson) as any);
  cachedConfigJson = configJson;
  return cachedProvider;
}

const STATIC_WASM_METHODS = new Set(["prove_private_note_inclusion_json"]);

async function runRequest(
  message: WasmWorkerRequest,
): Promise<SerializedWasmValue> {
  console.log(
    `[psy-diag] req id=${message.id} method=${message.method} mode=${message.mode ?? "provider"}`,
  );
  const provider = await ensureProviderReady(message.configJson);
  console.log("[psy-diag] provider ready (WASM server resolved, register_user dispatchable)");
  const args = (message.args ?? []).map(deserializeWasmValue);
  const t0 = Date.now();
  console.log(`[psy-diag] invoking ${message.method} (awaiting WASM result)…`);

  const heartbeat = setInterval(() => {
    console.log(`[psy-diag] still awaiting ${message.method} t=${Date.now() - t0}ms`);
  }, 2000);

  try {
    let result: unknown;
    if (message.mode === "static" || STATIC_WASM_METHODS.has(message.method)) {
      result = await PsyWasmWebProverProvider.runWasmServerCall((server) => {
        const fn = ((server as unknown) as Record<string, unknown>)[message.method];
        if (typeof fn !== "function") {
          throw new Error(`WASM static server does not expose ${message.method}`);
        }
        return (fn as (...a: unknown[]) => unknown).apply(server, args);
      });
    } else {
      const fn = (provider as unknown as Record<string, unknown>)[message.method];
      if (typeof fn !== "function") {
        const methods = Object.getOwnPropertyNames(
          Object.getPrototypeOf(provider),
        )
          .filter((m) => m !== "constructor")
          .slice(0, 30)
          .join(",");
        throw new Error(
          `WASM provider does not expose ${message.method} (surface: ${methods})`,
        );
      }
      result = await (fn as (...a: unknown[]) => unknown).apply(provider, args);
    }
    console.log(`[psy-diag] ${message.method} completed in ${Date.now() - t0}ms`);
    return serializeWasmValue(result);
  } finally {
    clearInterval(heartbeat);
  }
}

self.onmessage = (event: MessageEvent<WasmWorkerRequest>) => {
  const message = event.data;
  void (async () => {
    try {
      const result = await runRequest(message);
      (self as unknown as Worker).postMessage({ id: message.id, ok: true, result });
    } catch (error) {
      console.error(
        `[psy-diag] FAILED req id=${message.id} method=${message.method}`,
        error,
      );
      (self as unknown as Worker).postMessage({
        id: message.id,
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  })();
};

export {};
