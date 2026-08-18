/**
 * Browser-side privacy crypto primitives.
 *
 * Ported from the app's src/privacy/lib/crypto.ts. P2 needs only the two
 * identity-derivation functions (Nostr key + shield address); P3 expands this
 * module with the note/nullifier/deposit-commitment helpers as the private-
 * transfer stack lands. Kept byte-compatible with the rest of the Psy privacy
 * stack (same Poseidon + Nostr derivation) so on-chain artifacts match.
 */

import { getPublicKey, nip19 } from 'nostr-tools'
import { hashNoPad } from 'poseidon-goldilocks-lite'

export const SHIELD_ADDRESS_DOMAIN = 1337n

/**
 * Derive a Nostr keypair from a Psy private key + random0 + random1:
 *   nostr_sk = SHA-256("psy-nostr-v1:" + psyPrivateKeyHex + ":" + r0 + ":" + r1)
 * Deterministic: only the Psy key + the two scalars are needed to reproduce the
 * keypair, and r0/r1 double as the shield-address parameters.
 */
export async function deriveNostrKeyFromPsy(
  psyPrivateKeyHex: string,
  random0: string,
  random1: string,
): Promise<{ nsec: string; npub: string; sk: Uint8Array }> {
  const material = `psy-nostr-v1:${psyPrivateKeyHex}:${random0}:${random1}`
  const encoded = new TextEncoder().encode(material)
  const hashBuf = await crypto.subtle.digest('SHA-256', encoded)
  const sk = new Uint8Array(hashBuf) // 32 bytes → secp256k1 private key
  const pk = getPublicKey(sk)
  return {
    sk,
    nsec: nip19.nsecEncode(sk),
    npub: nip19.npubEncode(pk),
  }
}

/**
 * Shield (note_owner) address derivation (receiver side):
 *   note_owner = Poseidon(receiver_user_id, 1337, random0, random1)
 * Returns the 4×u64 limbs, or null if inputs are malformed.
 */
export function deriveShieldAddress(
  userId: string,
  random0: string,
  random1: string,
): [bigint, bigint, bigint, bigint] | null {
  try {
    const result = hashNoPad([
      BigInt(userId),
      SHIELD_ADDRESS_DOMAIN,
      BigInt(random0),
      BigInt(random1),
    ])
    return result as [bigint, bigint, bigint, bigint]
  } catch {
    return null
  }
}
