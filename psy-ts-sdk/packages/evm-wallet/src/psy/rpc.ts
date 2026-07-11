/**
 * JSON-RPC client for the Psy coordinator / realm / prove-proxy endpoints.
 *
 * Ported from the app's services/rpc.ts. The ONLY change: URLs come from the
 * resolved network (network.urls) instead of the chainConfig module singleton,
 * so the client is an instance bound to one client's config.
 */

import type { PsyNetworkUrls } from '../config/types'

// Read RPCs (coordinator/realm leaf reads) are fast; a HUNG node that
// TCP-accepts but never responds otherwise blocks the whole Activity feed/poll
// loop forever (no error, just stuck skeletons). Bound them. The prove-proxy is
// EXEMPT — its prove_* calls legitimately take 10–30s+ and must never be
// aborted by a read timeout.
const READ_RPC_TIMEOUT_MS = 15_000

export type UserLeaf = {
  balance: number
  nonce: number
  user_id: number
  public_key_hash: string
}

/** Realm id for a user id: floor(userId / users_per_realm). Pure — the app's
 *  getRealmIdFromUserId. */
export function realmIdFromUserId(userId: bigint | number, usersPerRealm: number): number {
  if (!usersPerRealm || usersPerRealm <= 0) return 0
  return Number(BigInt(userId) / BigInt(usersPerRealm))
}

export class RpcClient {
  constructor(
    private readonly urls: PsyNetworkUrls,
    private readonly usersPerRealm: number,
  ) {}

  private nextId = 1

  private async rpc<T>(
    url: string,
    method: string,
    params: Record<string, unknown> | unknown[] = [],
    timeoutMs?: number,
  ): Promise<T> {
    const ctrl = timeoutMs ? new AbortController() : null
    const timer = ctrl ? setTimeout(() => ctrl.abort(), timeoutMs) : null
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', id: this.nextId++, method, params }),
        signal: ctrl?.signal,
      })
      const json = await res.json()
      if (json.error) throw new Error(json.error.message ?? `RPC error for ${method}`)
      return json.result as T
    } finally {
      if (timer) clearTimeout(timer)
    }
  }

  realmIdFromUserId(userId: bigint | number): number {
    return realmIdFromUserId(userId, this.usersPerRealm)
  }

  coordinatorRpc<T = unknown>(
    method: string,
    params: Record<string, unknown> | unknown[] = [],
  ): Promise<T> {
    return this.rpc<T>(this.urls.coordinator, method, params, READ_RPC_TIMEOUT_MS)
  }

  realmRpc<T = unknown>(
    realmId: number,
    method: string,
    params: Record<string, unknown> | unknown[] = [],
  ): Promise<T> {
    return this.rpc<T>(this.urls.realm(realmId), method, params, READ_RPC_TIMEOUT_MS)
  }

  proveProxyRpc<T = unknown>(
    method: string,
    params: Record<string, unknown> | unknown[] = [],
  ): Promise<T> {
    // No timeout: real proving runs here and can take tens of seconds.
    return this.rpc<T>(this.urls.proveProxy, method, params)
  }

  getLatestCheckpointId(): Promise<number> {
    return this.coordinatorRpc<number>('psy_get_latest_checkpoint_id')
  }

  getUserLeafData(checkpointId: number, userId: number): Promise<UserLeaf> {
    return this.realmRpc<UserLeaf>(this.realmIdFromUserId(userId), 'psy_get_user_leaf_data', {
      checkpoint_id: checkpointId,
      user_id: userId,
    })
  }

  getUserTreeMerkleProof(checkpointId: number, userId: number): Promise<{ value: string }> {
    return this.realmRpc<{ value: string }>(
      this.realmIdFromUserId(userId),
      'psy_get_user_tree_merkle_proof',
      { checkpoint_id: checkpointId, user_id: userId },
    )
  }
}
