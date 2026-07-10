/**
 * @psy-protocol/evm-wallet — EVM wallet integration for the Psy network.
 *
 * The connected EVM wallet IS the key: sign-in and every operation (deposit,
 * withdraw, transfer, private transfer, claim, UPS proof sessions) are
 * authorized by the wallet's `personal_sign` (EIP-191) — no held key, no seed
 * phrase. ZK proofs are generated client-side (Web Worker WASM prover from the
 * peer @psy-protocol/psy-sdk) and submitted to the Psy network.
 *
 * Headless core. React bindings live at '@psy-protocol/evm-wallet/react'; the
 * prover worker module at '@psy-protocol/evm-wallet/worker'.
 */

export const EVM_WALLET_VERSION = '0.1.0';
