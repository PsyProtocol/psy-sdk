/**
 * Core Psy transaction types, ported from the app's privacy/types + psyClient.
 * These name the on-the-wire shapes the WASM prover methods take/return.
 */

/** One contract call the prover executes (privacy/types/index.ts). */
export interface ContractCallArgs {
  contract_id: bigint
  method_name: string
  inputs: bigint[]
}

/**
 * A single item in a claim batch (psyClient.ts). The wallet folds claims and an
 * optional public transfer into one array and submits via a single recursive
 * proof — one signature, one chain commit, atomic.
 */
export type ClaimBatchItem =
  | { type: 'public'; data: ContractCallArgs }
  | {
      type: 'private_transfer'
      data: {
        contract_id: string
        claim: {
          note_proof_bincode_b64: string
          nullifier: [string, string, string, string]
          owner: [string, string, string, string]
          amount: string
          user_tree_root: [string, string, string, string]
          checkpoint_id: string
          note_root_slot: string
          random0: string
          random1: string
          shield_address: string
        }
      }
    }
  | {
      type: 'claim_shield_deposit'
      data: {
        // Mirrors the wallet's buildImportedShieldDepositClaimBatchItem; the
        // wallet auto-injects r0/r1 from shield_address, so placeholder zeros for
        // random0/random1 are fine.
        nullifier: [string, string, string, string]
        note_secret_hash: [string, string, string, string]
        token_address_u32x8: string[]
        l2_token_contract_id: string[]
        amount_u32x8: string[]
        source_chain_index: string
        deposit_index: string
        deposit_root: [string, string, string, string]
        deposit_siblings: string[][]
        random0: string
        random1: string
        contract_id: string
        shield_address: string
      }
    }
