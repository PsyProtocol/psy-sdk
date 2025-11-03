import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

// Read WASM file
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const wasmPath = path.join(__dirname, './psy_prover_bg.wasm');
const wasmBuffer = fs.readFileSync(wasmPath);

// Convert to Uint8Array
const wasmArray = new Uint8Array(wasmBuffer);
const arrayString = Array.from(wasmArray).join(',');

// Generate TypeScript module
const moduleContent = `
// Auto-generated WASM binary data
// DO NOT EDIT MANUALLY

export const wasmBinary = new Uint8Array([${arrayString}]);

export default wasmBinary;
`;

// Write to output file
const outputPath = path.join(__dirname, './wasm-binary.ts');
fs.writeFileSync(outputPath, moduleContent);

console.log('WASM binary module generated successfully!');
console.log(`Size: ${wasmArray.length} bytes`);