/**
 * Mode-A submit orchestrator — the per-tx wallet-auth flow.
 *
 * Adapted for psy-sdk 2.0.4+ provider API. The secp256k1 key NEVER leaves
 * MetaMask; every register/send/claim is authorized by a personal_sign popup.
 * The Mode-A identity IS the wallet account key (pkHash = hash of its
 * secp256k1 pubkey).
 *
 * New SDK flow:
 *   Registration:
 *     ethPersonalRegistrationChallenge(evmAddress) -> challenge
 *       -> personal_sign(challenge)
 *       -> registerExternalEthPersonalUser(evmAddress, challenge, signature)
 *       -> pkHash
 *   Per-tx (exec / claim):
 *     generateTxTrace / generateBatchClaimTxTrace(pkHash, callData/claims)
 *       -> GeneratedTxTraceJson.sig_hash
 *       -> personal_sign(sigHash)
 *       -> injectEthPersonalSignature(pkHash, evmAddress, sigHash, signature)
 *       -> proveTxTrace(pkHash, generatedTrace)
 *       -> tx hash
 */

import type { ProverEngine } from '../prover/engine'
import type { EvmSigner } from '../evm/signer'
import type { ContractCallArgs, ClaimBatchItem } from '../psy/types'
import { assertRawPrehash, recoverCompressedPubkeyPersonalSign } from './mode-a-sig'

/** software_defined_call default — byte-identical to the ops layer's
 *  EMPTY_SIGN_DATA so the callData Mode-A signs over is the SAME envelope the
 *  chain proves. */
const EMPTY_SIGN_DATA = { inputs: [] as number[] }

/** Build the ContractCallData wrapper — MUST match the ops layer's wrapCallData
 *  exactly so generateTxTrace signs over the identical bytes execContractCall
 *  proves. */
function wrapCallData(calls: ContractCallArgs[]): {
  contract_calls: ContractCallArgs[]
  software_defined_call: { inputs: number[] }
} {
  return { contract_calls: calls, software_defined_call: EMPTY_SIGN_DATA }
}

type CallData = ReturnType<typeof wrapCallData>
type GeneratedTxTrace = { sig_hash: string; [key: string]: unknown }

export interface ModeAResult {
  txHash: string
  /** The compressed secp256k1 pubkey recovered from the personal_sign signature
   *  (0x-hex, 33 bytes). */
  recoveredPubkey: string
  /** The submitted tx's end-user-leaf hash (hex QHashOut), extracted from the
   *  generated trace. prove_private_note_inclusion_json's 9th param and
   *  wait_for_endcap_inclusion key on THIS value — the txHash proveTxTrace
   *  returns is a different hash, so private-transfer delivery must use this. */
  endUserLeafHash?: string
}

/** Pull finalization.submit_end_cap_input.core.state_transition.end_user_leaf_hash
 *  out of a GeneratedTxTraceJson envelope. Returns '' when absent (older wasm
 *  builds without the field). */
function extractEndUserLeafHash(trace: GeneratedTxTrace): string {
  try {
    const payload = (trace as { trace?: { payload?: string } }).trace?.payload
    if (typeof payload !== 'string' || payload === '') return ''
    const parsed = JSON.parse(payload) as {
      finalization?: {
        submit_end_cap_input?: {
          core?: { state_transition?: { end_user_leaf_hash?: unknown } }
        }
      }
    }
    const v = parsed.finalization?.submit_end_cap_input?.core?.state_transition?.end_user_leaf_hash
    return typeof v === 'string' ? v : ''
  } catch {
    return ''
  }
}

export class ModeASubmitter {
  constructor(
    private readonly prover: Pick<ProverEngine, 'callProver'>,
    private readonly signer: Pick<EvmSigner, 'ethPersonalSign'>,
  ) {}

  // ─── typed wrappers over the current SDK provider methods ───────────────────

  private ethPersonalRegistrationChallenge(evmAddress: string): Promise<string> {
    return this.prover.callProver<string>(
      'ethPersonalRegistrationChallenge',
      'ethPersonalRegistrationChallenge',
      [evmAddress],
    )
  }

  private registerExternalEthPersonalUser(
    evmAddress: string,
    challengeHex: string,
    signatureHex: string,
  ): Promise<string> {
    return this.prover.callProver<string>(
      'registerExternalEthPersonalUser',
      'registerExternalEthPersonalUser',
      [evmAddress, challengeHex, signatureHex],
    )
  }

  private generateTxTrace(pkHash: string, callData: CallData): Promise<GeneratedTxTrace> {
    return this.prover.callProver<GeneratedTxTrace>(
      'generateTxTrace',
      'generateTxTrace',
      [pkHash, callData],
    )
  }

  private generateBatchClaimTxTrace(pkHash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTrace> {
    return this.prover.callProver<GeneratedTxTrace>(
      'generateBatchClaimTxTrace',
      'generateBatchClaimTxTrace',
      [pkHash, claims],
    )
  }

  private injectEthPersonalSignature(
    pkHash: string,
    evmAddress: string,
    sigHash: string,
    signatureHex: string,
  ): Promise<string> {
    return this.prover.callProver<string>(
      'injectEthPersonalSignature',
      'injectEthPersonalSignature',
      [pkHash, evmAddress, sigHash, signatureHex],
    )
  }

  private proveTxTrace(pkHash: string, trace: GeneratedTxTrace): Promise<string> {
    return this.prover.callProver<string>('proveTxTrace', 'proveTxTrace', [pkHash, trace])
  }

  // ─── public submit flows ────────────────────────────────────────────────────

  /**
   * Submit one or more public contract calls under Mode-A: prove + auth via a
   * per-tx personal_sign, the secp256k1 key never leaving the wallet.
   */
  async execPublicContractCall(
    pkHash: string,
    calls: ContractCallArgs | ContractCallArgs[],
    ethAddress: string,
  ): Promise<ModeAResult> {
    const list = Array.isArray(calls) ? calls : [calls]
    if (list.length === 0) throw new Error('No contract calls to submit')
    const callData = wrapCallData(list)

    // 1. Generate the tx trace to obtain the raw 32-byte session sighash.
    const trace = await this.generateTxTrace(pkHash, callData)
    const sigHash = assertRawPrehash(trace.sig_hash)

    // 2. personal_sign the sighash (returns 65-byte r‖s‖v). >>> POPUP <<<
    const sig65 = await this.signer.ethPersonalSign(sigHash, ethAddress)

    // 3. Inject the external signature into the WASM server.
    await this.injectEthPersonalSignature(pkHash, ethAddress, sigHash, sig65)

    // 4. Prove the exact envelope that was signed; do not rebuild the trace.
    const txHash = await this.proveTxTrace(pkHash, trace)

    return {
      txHash: typeof txHash === 'string' ? txHash : String(txHash),
      recoveredPubkey: '0x' + recoverCompressedPubkeyPersonalSign(sigHash, sig65),
      endUserLeafHash: extractEndUserLeafHash(trace),
    }
  }

  /**
   * Submit a claim batch under Mode-A: prove the batch, then authorize it with a
   * per-tx personal_sign over the claim-session sighash.
   */
  async claimBatch(
    pkHash: string,
    claims: ClaimBatchItem[],
    ethAddress: string,
  ): Promise<ModeAResult> {
    if (claims.length === 0) throw new Error('No claim items to submit')

    const trace = await this.generateBatchClaimTxTrace(pkHash, claims)
    const sigHash = assertRawPrehash(trace.sig_hash)
    const sig65 = await this.signer.ethPersonalSign(sigHash, ethAddress)
    await this.injectEthPersonalSignature(pkHash, ethAddress, sigHash, sig65)
    const txHash = await this.proveTxTrace(pkHash, trace)

    return {
      txHash: typeof txHash === 'string' ? txHash : String(txHash),
      recoveredPubkey: '0x' + recoverCompressedPubkeyPersonalSign(sigHash, sig65),
      endUserLeafHash: extractEndUserLeafHash(trace),
    }
  }

  /**
   * Mode-A registration: recover the wallet's secp256k1 PUBLIC key from ONE
   * personal_sign, then register that public key as a Psy account with NO held
   * private key. Returns the pkHash + resolved user_id (null until it lands
   * on-chain — the caller polls).
   */
  async registerUser(
    ethAddress: string,
  ): Promise<{ pkHash: string; userId: string | null; recoveredPubkey: string }> {
    const challenge = await this.ethPersonalRegistrationChallenge(ethAddress)
    const digest = assertRawPrehash(challenge)
    const sig65 = await this.signer.ethPersonalSign(digest, ethAddress)
    const pkHash = await this.registerExternalEthPersonalUser(ethAddress, digest, sig65)

    return {
      pkHash: typeof pkHash === 'string' ? pkHash : String(pkHash),
      userId: null,
      recoveredPubkey: '0x' + recoverCompressedPubkeyPersonalSign(digest, sig65),
    }
  }
}
