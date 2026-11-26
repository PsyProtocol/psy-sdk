// Export the generator tools
export { SDKGenerator } from "./generator";
export { AbiConverter } from "./converters/abi-converter";

// Export runtime module
export { Contract } from "./runtime/contract";
export { Signer } from "./runtime/types";
export type { Felt, ISigner, IContractProvider, IContractStateReader, PsyFixedArray, ContractOptions } from "./runtime";

// Only export generated files if they exist.
// This allows the package to build before generation has run.
let generatedPath: string | undefined;
try {
    // @ts-ignore - Generated files may not exist yet
    generatedPath = require.resolve("../generated");
} catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "MODULE_NOT_FOUND") {
        throw e;
    }
}

if (generatedPath) {
    // @ts-ignore - Generated files may not exist yet
    module.exports.generated = require(generatedPath);
}

// Export types that will be available after generation
export type { AbiInput, Abi } from "./types/abi-format";
export { isAbi } from "./types/abi-format";
