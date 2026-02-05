import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_prover_bg.wasm");

function runWasmPack(target: "web" | "nodejs", outDir: string, rustSdkDir: string) {
    const args = ["build", "--target", target, "--out-dir", outDir, "--out-name", "psy_prover", "--no-pack", "--release"];
    const result = spawnSync("wasm-pack", args, {
        cwd: rustSdkDir,
        stdio: "inherit",
        env: process.env,
    });

    if (result.status !== 0) {
        throw new Error(`wasm-pack build failed for target "${target}"`);
    }
}

function ensureWasmArtifacts() {
    if (fs.existsSync(wasmPath)) {
        return;
    }

    console.log("WASM binary not found. Building via wasm-pack...");

    const rustSdkDir = path.resolve(__dirname, "../../../../../psy-rust-sdk");
    const webOutDir = path.resolve(__dirname);
    const nodeOutDir = path.resolve(__dirname, "../local-prover");

    runWasmPack("web", webOutDir, rustSdkDir);
    runWasmPack("nodejs", nodeOutDir, rustSdkDir);

    if (!fs.existsSync(wasmPath)) {
        throw new Error(`Expected wasm artifact at ${wasmPath}, but it was not generated`);
    }
}

ensureWasmArtifacts();

const wasmBuffer = fs.readFileSync(wasmPath);

// Convert to Uint8Array
const wasmArray = new Uint8Array(wasmBuffer);
const arrayString = Array.from(wasmArray).join(',');

// Generate TypeScript module
const moduleContent = `
// Auto-generated WASM binary data
// DO NOT EDIT MANUALLY

export const wasmBinary = new Uint8Array([${arrayString}]);
`;

// Write to output file
const outputPath = path.join(__dirname, './wasm-binary.ts');
fs.writeFileSync(outputPath, moduleContent);

console.log('WASM binary module generated successfully!');
console.log(`Size: ${wasmArray.length} bytes`);
