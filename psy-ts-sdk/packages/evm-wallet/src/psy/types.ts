/**
 * Core Psy transaction types.
 *
 * These are RE-EXPORTED from @psy-protocol/psy-sdk (not redefined) so the
 * module, the SDK, and the Bridge App all reference byte-identical shapes — the
 * signing swap then has zero type friction. The SDK is the source of truth:
 *   ContractCallArgs  = { contract_id: bigint, method_name: string, inputs: bigint[] }
 *   SignData          = { inputs: number[] }
 *   ContractCallData  = { contract_calls: ContractCallArgs[], software_defined_call: SignData }
 *   ClaimBatchItem    = public | private_transfer | claim_shield_deposit union
 */

export type {
  ContractCallArgs,
  ContractCallData,
  SignData,
  ClaimBatchItem,
} from '@psy-protocol/psy-sdk'
