const fs = require('fs');
const path = require('path');

// Read WASM file
const wasmPath = path.join(__dirname, './qed_user_prover_bg.wasm');
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