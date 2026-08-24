/**
 * ProverEngine — one instance owns the entire prover pipeline for a client:
 *
 *   callProver(label, method, args)
 *     = runExclusive(label, () => runWorkerCall(method, configJson, args))
 *
 * It fuses three modules that were module-scoped singletons in the mode-a app
 * (unified/prover-client.ts + lib/wasm-exclusive.ts + unified/gated-prover.ts)
 * into instance state, so two clients on one page never share a worker, a FIFO
 * queue, or a signer registry. Behavior is otherwise byte-identical to the app:
 *
 * - FIFO gate: serializes ALL prover calls (prevents the Rust "recursive use of
 *   an object" borrow panic under concurrent register/execContractCall/
 *   claimBatch). wasm-bindgen async methods hold a &mut borrow of the
 *   WasmRpcServer for their whole future; any second call landing in that window
 *   throws synchronously at the glue's borrow check.
 * - Worker: runs the actual WASM on a dedicated thread so a multi-second proof
 *   never freezes the UI. Spawned lazily, respawned on crash; a per-call
 *   watchdog fails a wedged call so the gate/UI can't hang forever.
 * - Dispatch modes: "provider" (default) for provider-instance methods, "static"
 *   for PsyWasmWebProverProvider.wasmServer methods (prove_private_note_-
 *   inclusion_json). Both run on the SAME worker, so a static proof signs with
 *   the registry registerUser installed.
 */

import { PsyJSON } from '@psy-protocol/psy-sdk'
import {
  deserializeWasmValue,
  serializeWasmValue,
  type SerializedWasmValue,
  type WasmWorkerReply,
} from './wasm-codec'

/**
 * Upper bound on a single prover-worker call. Generous on purpose: the FIRST
 * call also pays the in-browser WASM circuit build (several seconds × multiple
 * circuits), and real Plonky2 proofs take ~10–30s. So this only fires when the
 * worker is genuinely WEDGED (never posts a reply and never throws onerror).
 */
const PROVER_CALL_TIMEOUT_MS = 300_000

interface PendingCall {
  resolve: (value: unknown) => void
  reject: (reason: Error) => void
  timer?: ReturnType<typeof setTimeout>
}

export class ProverEngine {
  /** How this engine gets a fresh prover Worker. Injected (app-local 1-line file
   *  that imports '@psy-protocol/evm-wallet/worker') or the dist fallback. */
  private readonly createWorker: () => Worker
  /** Memoized PsyJSON string of the active PsyNetworkConfig (bigint-safe). */
  private readonly configJson: string

  private worker: Worker | undefined
  private nextRequestId = 1
  private readonly pending = new Map<number, PendingCall>()

  // FIFO gate state (was lib/wasm-exclusive module scope).
  private queue: Promise<void> = Promise.resolve()

  constructor(input: { psyConfig: unknown; createWorker?: () => Worker }) {
    this.configJson = PsyJSON.stringify(input.psyConfig)
    this.createWorker = input.createWorker ?? (() => this.spawnFallbackWorker())
  }

  /**
   * Best-effort dist fallback when no createWorker is injected: spawn the built
   * worker.mjs that sits one level up from this engine.mjs (dist/prover/ →
   * dist/worker.mjs).
   *
   * The specifier is assembled at RUNTIME (not a bare string literal) ON PURPOSE:
   * bundlers (Vite's worker-import-meta-url, Rollup) statically match
   * `new Worker(new URL('<literal>', import.meta.url))` and would try to bundle
   * this fallback into the CONSUMER's graph — pulling the 22 MB WASM in twice and
   * failing to resolve the path. The canonical, bundler-proof path is the
   * injected createWorker factory (a 1-line app-local `import '.../worker'`
   * file); this fallback only runs for consumers who don't inject one, and needs
   * the bundler to leave the library worker un-prebundled (Vite:
   * optimizeDeps.exclude).
   */
  private spawnFallbackWorker(): Worker {
    const spec = ['..', 'worker.mjs'].join('/')
    return new Worker(new URL(spec, import.meta.url), { type: 'module' })
  }

  // ── worker lifecycle (was unified/prover-client.ts) ─────────────────────────

  private takePending(id: number): PendingCall | undefined {
    const entry = this.pending.get(id)
    if (!entry) return undefined
    this.pending.delete(id)
    if (entry.timer) clearTimeout(entry.timer)
    return entry
  }

  private ensureWorker(): Worker {
    if (this.worker) return this.worker
    let w: Worker
    try {
      w = this.createWorker()
    } catch (e) {
      // Spawn failed (CSP/module-load/OOM). Leave worker undefined so the next
      // call retries a fresh spawn, and surface a classifiable message ("proof"
      // → proof_failed) naming createWorker so the fix is obvious.
      throw new Error(
        `proof_failed: prover failed to start — ${(e as Error)?.message ?? String(e)}. ` +
          'If bundling this library, pass createWorker to createPsyWallet (see README).',
      )
    }
    w.onmessage = (event: MessageEvent<WasmWorkerReply>) => {
      const { id, ok, result, error } = event.data
      const entry = this.takePending(id)
      if (!entry) return
      if (ok) {
        entry.resolve(
          result === undefined ? undefined : deserializeWasmValue(result as SerializedWasmValue),
        )
      } else {
        console.error(`[psy-diag] worker reply FAILED id=${id}: ${error || 'unknown error'}`)
        entry.reject(new Error(error || 'Prover worker call failed'))
      }
    }
    w.onerror = (event) => {
      const message = event.message || 'Prover WASM worker crashed'
      console.error('[psy-diag] prover worker crashed', {
        message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
        error: event.error,
      })
      for (const id of [...this.pending.keys()]) {
        this.takePending(id)?.reject(new Error(message))
      }
      if (this.worker === w) this.worker = undefined
    }
    this.worker = w
    return w
  }

  /** Spawn the worker if needed so the FIFO gate can warm it inside its slot. */
  async ensureReady(): Promise<void> {
    this.ensureWorker()
  }

  /**
   * Tear down the worker and reject anything in flight. Called on logout so the
   * next session starts from a fresh SDK wasmServer signer registry — the prior
   * account's signer material does not linger, and the registry can't grow
   * unbounded over repeated logins. Safe when no worker exists (no-op).
   */
  terminate(): void {
    const w = this.worker
    this.worker = undefined
    for (const id of [...this.pending.keys()]) {
      this.takePending(id)?.reject(new Error('Prover worker terminated'))
    }
    if (w) w.terminate()
  }

  private runWorkerCall<T>(
    method: string,
    args: unknown[],
    mode: 'provider' | 'static',
    timeoutMs: number = PROVER_CALL_TIMEOUT_MS,
    timeoutMessage = 'proof_failed: proof generation timed out — refresh and try again',
  ): Promise<T> {
    const id = this.nextRequestId++
    const w = this.ensureWorker()
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        const timedOut = this.takePending(id)
        if (!timedOut) return

        // A timed-out wasm-bindgen future may still be running inside the
        // worker. Rejecting only the caller leaves that future holding the
        // WasmRpcServer's mutable borrow, so the FIFO queue can dispatch its
        // next request into a permanently wedged prover. Treat a timeout as a
        // worker-level circuit breaker: discard the whole worker and let the
        // next gated call create a fresh WASM server.
        const wedgedWorker = this.worker
        this.worker = undefined
        wedgedWorker?.terminate()

        timedOut.reject(new Error(timeoutMessage))

        // FIFO normally means there is only one pending worker call. Clear any
        // stragglers defensively so no promise remains attached to the worker
        // we just terminated.
        for (const pendingId of [...this.pending.keys()]) {
          this.takePending(pendingId)?.reject(
            new Error('Prover worker restarted after a timed-out request'),
          )
        }
      }, timeoutMs)
      this.pending.set(id, { resolve: resolve as (v: unknown) => void, reject, timer })
      try {
        w.postMessage({
          id,
          mode,
          method,
          configJson: this.configJson,
          args: args.map(serializeWasmValue),
        })
      } catch (e) {
        this.takePending(id)?.reject(e instanceof Error ? e : new Error(String(e)))
      }
    })
  }

  // ── FIFO gate (was lib/wasm-exclusive.ts) ──────────────────────────────────

  /** Run `fn` as the sole owner of the WasmRpcServer: FIFO behind every other
   *  wasm-touching call, warming the worker first. Failures release the queue
   *  for the next caller (never poisoned). `fn` must be a LEAF (never re-enter). */
  private runExclusive<T>(_label: string, fn: () => Promise<T>): Promise<T> {
    const run = this.queue.then(async () => {
      await this.ensureReady()
      return await fn()
    })
    this.queue = run.then(
      () => undefined,
      () => undefined,
    )
    return run
  }

  // ── public surface (was unified/gated-prover.ts) ────────────────────────────

  /** Run one provider-instance method, gated + workerized (execContractCall,
   *  claimBatch, registerUser, …). The ONLY way to touch the provider instance. */
  callProver<T = unknown>(label: string, method: string, args: unknown[]): Promise<T> {
    return this.runExclusive(label, () => this.runWorkerCall<T>(method, args, 'provider'))
  }

  /** Run one method on the STATIC wasmServer, gated + workerized. Same FIFO choke
   *  point as callProver (so a static note-inclusion proof and a concurrent
   *  execContractCall can't trigger the recursive-borrow panic).
   *  prove_private_note_inclusion_json is the sole caller. */
  callProverStatic<T = unknown>(label: string, method: string, args: unknown[]): Promise<T> {
    return this.runExclusive(label, () => this.runWorkerCall<T>(method, args, 'static'))
  }
}
