/**
 * PsyWalletRuntime — the per-client instance context.
 *
 * Everything that was a module-scoped singleton in the mode-a app (prover
 * worker handle + pending map, FIFO wasm gate, Mode-A pkHash→ethAddress
 * registry, claim idempotency guards, tracker handles, config JSON cache)
 * becomes a field here, threaded through the ported modules. Two clients in
 * one page therefore never share prover or identity state.
 *
 * Fields are populated by their owning phases (P2 prover/session, P3 ops,
 * P4 L1 seam, P5 trackers); this file only defines the container so every
 * ported module has a stable seam from day one.
 */

import type { Config as WagmiConfig } from 'wagmi'
import type { PsyStorage, ResolvedNetwork } from './config/types'

/** Simple typed event hub (no dependency; mirrors the app's window-event use). */
export class Emitter<Events extends Record<string, unknown>> {
  private handlers = new Map<keyof Events, Set<(payload: never) => void>>()

  on<K extends keyof Events>(event: K, handler: (payload: Events[K]) => void): () => void {
    let set = this.handlers.get(event)
    if (!set) {
      set = new Set()
      this.handlers.set(event, set)
    }
    set.add(handler as (payload: never) => void)
    return () => set?.delete(handler as (payload: never) => void)
  }

  emit<K extends keyof Events>(event: K, payload: Events[K]): void {
    this.handlers.get(event)?.forEach((h) => {
      try {
        ;(h as (payload: Events[K]) => void)(payload)
      } catch {
        // A listener throwing must never break the emit loop.
      }
    })
  }
}

/** Events the runtime broadcasts (grown by later phases). */
export interface RuntimeEvents extends Record<string, unknown> {
  /** Any persisted transaction changed — feeds activity/inbox refreshes. */
  'transactions-changed': undefined
}

const memoryStorage = (): PsyStorage => {
  const m = new Map<string, string>()
  return {
    get: (k) => m.get(k) ?? null,
    set: (k, v) => void m.set(k, v),
    remove: (k) => void m.delete(k),
  }
}

const defaultStorage = (): PsyStorage => {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      return {
        get: (k) => window.localStorage.getItem(k),
        set: (k, v) => window.localStorage.setItem(k, v),
        remove: (k) => window.localStorage.removeItem(k),
      }
    }
  } catch {
    // storage unavailable (privacy mode / SSR) — fall through
  }
  return memoryStorage()
}

export interface PsyWalletRuntime {
  readonly network: ResolvedNetwork
  readonly wagmiConfig: WagmiConfig
  readonly storage: PsyStorage
  readonly createWorker?: () => Worker
  readonly events: Emitter<RuntimeEvents>
  /** Memoized PsyJSON config string for prover construction (set in P2). */
  proverConfigJson?: string
}

export function createRuntime(input: {
  network: ResolvedNetwork
  wagmiConfig: WagmiConfig
  storage?: PsyStorage
  createWorker?: () => Worker
}): PsyWalletRuntime {
  return {
    network: input.network,
    wagmiConfig: input.wagmiConfig,
    storage: input.storage ?? defaultStorage(),
    createWorker: input.createWorker,
    events: new Emitter<RuntimeEvents>(),
  }
}
