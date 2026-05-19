import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_compiler_bg.wasm");

function runWasmPack(target: "web" | "nodejs", outDir: string, psyCompilerDir: string) {
    const args = ["build", "psy-wasm", "--target", target, "--out-dir", outDir, "--out-name", "psy_compiler", "--no-pack", "--release"];
    const result = spawnSync("wasm-pack", args, {
        cwd: psyCompilerDir,
        stdio: "inherit",
        env: process.env,
    });

    if (result.status !== 0) {
        throw new Error(`wasm-pack build failed for target "${target}"`);
    }
}

function ensureWasmArtifacts() {
    console.log("Building compiler WASM artifacts via wasm-pack...");

    const psyCompilerDir = path.resolve(__dirname, "../../../../../../psy-compiler");
    const webOutDir = path.resolve(__dirname);

    runWasmPack("web", webOutDir, psyCompilerDir);

    const gitignoreTemplate = path.resolve(__dirname, "../../../../../.github/templates/.gitignore.wasm");
    if (fs.existsSync(gitignoreTemplate)) {
        fs.copyFileSync(gitignoreTemplate, path.join(webOutDir, ".gitignore"));
    }

    if (!fs.existsSync(wasmPath)) {
        throw new Error(`Expected wasm artifact at ${wasmPath}, but it was not generated`);
    }
}

ensureWasmArtifacts();

const wasmBuffer = fs.readFileSync(wasmPath);
const wasmArray = new Uint8Array(wasmBuffer);
const arrayString = Array.from(wasmArray).join(",");

const moduleContent = `
// Auto-generated WASM binary data
// DO NOT EDIT MANUALLY

export const wasmBinary = new Uint8Array([${arrayString}]);
`;

const outputPath = path.join(__dirname, "./wasm-binary.ts");
fs.writeFileSync(outputPath, moduleContent);

console.log("Compiler WASM binary module generated successfully!");
console.log(`Size: ${wasmArray.length} bytes`);
