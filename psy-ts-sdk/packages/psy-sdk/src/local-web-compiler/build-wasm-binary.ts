import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_compiler_bg.wasm");

function runWasmPack(target: "web" | "nodejs", outDir: string, psyCompilerDir: string) {
    const wasmCrateDir = fs.existsSync(path.join(psyCompilerDir, "psy-wasm", "Cargo.toml"))
        ? "psy-wasm"
        : "psy_wasm";
    const args = ["build", wasmCrateDir, "--target", target, "--out-dir", outDir, "--out-name", "psy_compiler", "--no-pack", "--release"];
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
    if (process.env.SKIP_WASM_PACK) {
        console.log("SKIP_WASM_PACK set: re-encoding existing compiler WASM, skipping wasm-pack...");
        if (!fs.existsSync(wasmPath)) {
            throw new Error(`SKIP_WASM_PACK set but no wasm artifact found at ${wasmPath}`);
        }
        return;
    }

    console.log("Building compiler WASM artifacts via wasm-pack...");

    const envCompilerDir = process.env.PSY_COMPILER_DIR?.trim();
    const defaultParthCompilerDir = path.resolve(
        __dirname,
        "../../../../../../parth-generic-v1/client_prover/psy_ide",
    );
    const legacyCompilerDir = path.resolve(__dirname, "../../../../../../psy-compiler");
    const psyCompilerDir = envCompilerDir
        ? path.resolve(envCompilerDir)
        : (fs.existsSync(legacyCompilerDir) ? legacyCompilerDir : defaultParthCompilerDir);
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
const base64Chunks = wasmBuffer.toString("base64").match(/.{1,65536}/g) ?? [];
const base64ChunkString = base64Chunks.map((chunk) => `    ${JSON.stringify(chunk)},`).join("\n");

const moduleContent = `
// Auto-generated WASM binary data
// DO NOT EDIT MANUALLY

const base64Chunks: string[] = [
${base64ChunkString}
];

function decodeBase64(chunks: string[]): Uint8Array {
    const base64 = chunks.join("");
    const atob = (globalThis as { atob?: (data: string) => string }).atob;

    if (typeof atob === "function") {
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);

        for (let i = 0; i < binary.length; i += 1) {
            bytes[i] = binary.charCodeAt(i);
        }

        return bytes;
    }

    const buffer = (globalThis as {
        Buffer?: { from(data: string, encoding: "base64"): Uint8Array };
    }).Buffer;

    if (buffer) {
        return new Uint8Array(buffer.from(base64, "base64"));
    }

    throw new Error("No base64 decoder is available in this environment");
}

export const wasmBinary = decodeBase64(base64Chunks);
`;

const outputPath = path.join(__dirname, "./wasm-binary.ts");
fs.writeFileSync(outputPath, moduleContent);

console.log("Compiler WASM binary module generated successfully!");
console.log(`Size: ${wasmBuffer.length} bytes`);
