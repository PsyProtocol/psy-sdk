// Export new types and implementations
export * from "./types";
export * from "./client";

// Re-export related types for backward compatibility
import { QEDRPCUserProverProvider } from "./client";
import { ContractCallArgs, IQEDUserProverProvider, QEDUserProverRPCCommand, WalletKeyPair } from "./types";

export { QEDUserProverRPCCommand, QEDRPCUserProverProvider };
export type { ContractCallArgs, WalletKeyPair, IQEDUserProverProvider };
