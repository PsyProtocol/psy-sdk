// Export all types
export * from "./types";
export * from "./client";

// Re-export the main client class with an alias for backward compatibility if needed
import { RealmEdgeRpcProvider } from "./client";
export { RealmEdgeRpcProvider as QEDRealmEdgeRpcProvider };
