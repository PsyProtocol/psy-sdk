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

export { QEDUserProverRPCCommand, QEDRPCUserProverProvider };
export type { ContractCallArgs, WalletKeyPair, ZKPublicKeyInfo, IQEDUserProverProvider };
