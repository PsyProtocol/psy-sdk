// Export the generator tools
export { SDKGenerator } from "./generator";
export { AbiConverter } from "./converters/abi-converter";

// Export runtime module
export { Contract } from "./runtime/contract";
export { Signer } from "./runtime/types";
export type { Felt, ISigner, PsyFixedArray, ContractOptions } from "./runtime";

// Only try to export generated files if they exist
// This allows the package to build even before generation
try {
    // @ts-ignore - Generated files may not exist yet
    module.exports.generated = require("../generated");
} catch (e) {
    // Generated files don't exist yet, that's OK
}

// Export types that will be available after generation
export type { AbiFormat } from "./types/abi-format";
