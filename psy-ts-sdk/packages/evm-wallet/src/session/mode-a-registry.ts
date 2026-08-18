/**
 * Mode-A eth-address registry — maps a Psy public-key HASH to the EVM account
 * that authorizes its transactions via personal_sign.
 *
 * Ported from the app's unified/mode-a-registry.ts; the module-scoped Map is now
 * an instance field so two clients never cross bindings. THE LOAD-BEARING SEAM
 * for TRUE Mode-A: the ops layer takes a pkHash but NOT the EVM address, and
 * looks the address up here to decide the submit path:
 *
 *   - pkHash IS registered  → an EVM-account identity: authorize the call/claim
 *     through Mode-A (getSigHash → personal_sign → *WithExternalSignature). The
 *     secp256k1 key is NEVER held — every op pops a wallet signature prompt.
 *   - pkHash is NOT registered → a held-key signer installed in the worker (e.g.
 *     a faucet SDK-key operator): keep the legacy held-key submit path.
 *
 * Cleared on logout/account-switch (reset), so a stale address can never sign
 * for a new identity.
 */
export class ModeARegistry {
  private readonly pkHashToEthAddress = new Map<string, string>()

  /** Normalize a pkHash to a stable lookup key (bare lowercase hex). */
  private static norm(pkHash: string): string {
    return pkHash.replace(/^0x/i, '').toLowerCase()
  }

  /** Bind a Psy public-key hash to the EVM account that authorizes it. */
  register(pkHash: string, ethAddress: string): void {
    this.pkHashToEthAddress.set(ModeARegistry.norm(pkHash), ethAddress.toLowerCase())
  }

  /** The EVM address that authorizes this pkHash via personal_sign, or null when
   *  the pkHash is a held-key signer (operator) that uses the legacy path. */
  ethAddressForPkHash(pkHash: string): string | null {
    return this.pkHashToEthAddress.get(ModeARegistry.norm(pkHash)) ?? null
  }

  /** True when this pkHash is a Mode-A (EVM-authorized, no held key) identity. */
  isModeAIdentity(pkHash: string): boolean {
    return this.pkHashToEthAddress.has(ModeARegistry.norm(pkHash))
  }

  /** Clear all bindings (logout / account switch). */
  reset(): void {
    this.pkHashToEthAddress.clear()
  }
}
