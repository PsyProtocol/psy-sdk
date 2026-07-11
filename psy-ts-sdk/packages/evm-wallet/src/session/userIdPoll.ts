/**
 * Bounded, checkpoint-aware poll for the on-chain userId of a registered key.
 *
 * Lives in its OWN module (no chain-config / rpc imports) so the pure timing
 * logic is unit-testable under bun without dragging in Vite-only
 * `import.meta.glob` deployment loading. session.ts injects the real seams.
 *
 * The race this solves: a freshly registered secp256k1 key isn't indexed into a
 * checkpoint for ~2 checkpoints (~10-30s each). A fixed short retry exhausts long
 * before the realm sees it and mis-surfaces a "busy" error on every FIRST
 * sign-in. This poll waits across the checkpoint budget instead — while keeping
 * the returning-user path instant (attempt 0).
 */

/** Progress info emitted while we wait for a fresh registration to land in a
 *  checkpoint. The UI renders a calm "finishing registration on-chain…" state
 *  off these — it is NOT an error. */
export interface UserIdPollProgress {
  elapsedMs: number
  checkpointsAdvanced: number
}

/** Injectable seams for `pollUserIdAcrossCheckpoints`. Tests pass fakes so the
 *  poll runs in milliseconds with no network and no global mutation. */
export interface UserIdPollDeps {
  /** Resolve the userId for a pk hash, or null if not yet indexed. */
  resolve: (pkHash: string) => Promise<string | null>
  /** Read the latest checkpoint id (may throw — caller falls back to wall-clock). */
  getCheckpoint: () => Promise<number>
  /** Sleep `ms` milliseconds. */
  sleep: (ms: number) => Promise<void>
  /** Monotonic-ish current time in ms. */
  now: () => number
}

/** Dual-guard budget. A freshly-registered key resolves once the register-user
 *  gatherer finalizes it into a checkpoint. `resolve()` returns IMMEDIATELY on
 *  success, so a returning user or a fast chain never waits — these caps only
 *  bound how long we tolerate a SLOW registration before showing the "still
 *  finalizing" retry state.
 *
 *  The checkpoint guard is deliberately generous: a local devnet emits EMPTY
 *  checkpoints every ~1-2s while a real registration can take several minutes to
 *  gather+finalize, so a tiny checkpoint budget (the old value 3) tripped in
 *  seconds and failed sign-in on a chain that was working fine. The wall-clock
 *  cap is the real guard now; the checkpoint count only protects against a
 *  wedged chain that stops advancing entirely. Raising these is safe — the fast
 *  path is unaffected because resolve() short-circuits. */
// Effectively disabled: this devnet fires empty checkpoints in BURSTS (observed
// jumping thousands of checkpoints in seconds while idle), so ANY finite
// checkpoint budget trips long before a ~10-min register finalizes. Wall-clock
// is the only reliable guard here; keep a huge value only to catch a fully
// wedged chain. resolve() short-circuits on success so the fast path is instant.
export const POLL_CHECKPOINT_BUDGET = 10_000_000
export const POLL_WALLCLOCK_CAP_MS = 900_000
export const POLL_INTERVAL_MS = 3_000

/** The exhaustion message. MUST contain "busy" so classifyLoginError() maps it
 *  to `sign_in_busy` (a calm retry, not a hard failure) for the genuine
 *  chain-is-really-slow-today tail case. */
export const POLL_BUSY_MESSAGE =
  'The registration service is busy finishing your sign-in — your key may be registered; try again in a moment.'

/**
 * Bounded, checkpoint-aware poll for the on-chain userId of a pk hash.
 *
 *  - Attempt 0 is IMMEDIATE: a returning (already-indexed) key resolves with
 *    zero added latency — the non-negotiable instant path.
 *  - If unresolved, poll every `POLL_INTERVAL_MS` until the userId resolves, OR
 *    `POLL_CHECKPOINT_BUDGET` checkpoints have landed since the start, OR the
 *    `POLL_WALLCLOCK_CAP_MS` wall-clock cap is hit. Checkpoint reads are wrapped
 *    so a failing/stalled checkpoint RPC degrades to wall-clock only (never hangs).
 *  - On exhaustion, throws an error whose message contains "busy" so the calm
 *    retry path is preserved.
 */
export async function pollUserIdAcrossCheckpoints(
  pkHash: string,
  onProgress: ((info: UserIdPollProgress) => void) | undefined,
  deps: UserIdPollDeps,
): Promise<string> {
  const start = deps.now()

  // Attempt 0 — immediate. Returning users return here with no sleep, no
  // checkpoint read.
  const first = await deps.resolve(pkHash)
  if (first) return first

  // Capture the starting checkpoint as the baseline for the checkpoint budget.
  // If the read fails, we run on the wall-clock guard alone.
  let startCheckpoint: number | null = null
  try {
    startCheckpoint = await deps.getCheckpoint()
  } catch {
    startCheckpoint = null
  }

  for (;;) {
    await deps.sleep(POLL_INTERVAL_MS)

    const userId = await deps.resolve(pkHash)
    if (userId) return userId

    const elapsedMs = deps.now() - start

    // Count checkpoint advancement (primary guard). Tolerate read failures.
    let checkpointsAdvanced = 0
    if (startCheckpoint != null) {
      try {
        const latest = await deps.getCheckpoint()
        checkpointsAdvanced = Math.max(0, latest - startCheckpoint)
      } catch {
        // Checkpoint read failed mid-poll — keep going on the wall-clock guard.
        checkpointsAdvanced = 0
      }
    }

    onProgress?.({ elapsedMs, checkpointsAdvanced })

    if (checkpointsAdvanced >= POLL_CHECKPOINT_BUDGET) break
    if (elapsedMs >= POLL_WALLCLOCK_CAP_MS) break
  }

  // Registered (or will be), but not yet indexed into a checkpoint. Funds/
  // identity are unaffected; a refresh moments later resolves the id.
  throw new Error(POLL_BUSY_MESSAGE)
}
