// Export new types and implementations
export * from "./qedTypes";
export * from "./qedClient";

// Re-export related types for backward compatibility
import { QEDRPCUserProverProvider } from "./qedClient";
import {
    ContractCallArgs,
    IQEDUserProverProvider,
    QEDUserProverRPCCommand,
    WalletKeyPair,
    ZKPublicKeyInfo,
} from "./qedTypes";

export { QEDUserProverRPCCommand };
export type { ContractCallArgs, WalletKeyPair, ZKPublicKeyInfo };
export { QEDRPCUserProverProvider };
export type { IQEDUserProverProvider };

