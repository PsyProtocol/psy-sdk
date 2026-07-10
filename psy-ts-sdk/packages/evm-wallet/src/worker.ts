/**
 * Prover Web Worker entry ('@psy-protocol/evm-wallet/worker').
 *
 * Side-effectful by design: importing this module inside a Worker registers
 * `self.onmessage` and hosts the WASM prover (PsyWasmWebProverProvider from the
 * peer @psy-protocol/psy-sdk — the WASM binary is never duplicated here).
 *
 * Consumers spawn it via a 1-line app-local worker file so THEIR bundler sees
 * a first-party URL (the canonical, bundler-proof path):
 *
 *   // src/psy-prover.worker.ts (in your app)
 *   import '@psy-protocol/evm-wallet/worker'
 *
 *   createPsyWallet({ ..., createWorker: () =>
 *     new Worker(new URL('./psy-prover.worker.ts', import.meta.url), { type: 'module' }) })
 *
 * The full worker implementation is ported in phase P2 (from the mode-a app's
 * src/unified/prover-worker.ts, verbatim — including the async-WasmRpcServer
 * re-bind and diagnostic heartbeats).
 */

export {};
