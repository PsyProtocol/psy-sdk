import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_prover_bg.wasm");

function runWasmPack(target: "web" | "nodejs", outDir: string, rustSdkDir: string) {
    const args = [
        "build",
        "--target",
        target,
        "--out-dir",
        outDir,
        "--out-name",
        "psy_prover",
        "--no-pack",
        "--release",
        "--no-opt",
    ];
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
    if (process.env.SKIP_WASM_PACK) {
        console.log("SKIP_WASM_PACK set: re-encoding existing prover WASM, skipping wasm-pack...");
        if (!fs.existsSync(wasmPath)) {
            throw new Error(`SKIP_WASM_PACK set but no wasm artifact found at ${wasmPath}`);
        }
        return;
    }

    console.log("Building prover WASM artifacts via wasm-pack...");

    const rustSdkDir = path.resolve(__dirname, "../../../../../psy-rust-sdk");
    const webOutDir = path.resolve(__dirname);
    const nodeOutDir = path.resolve(__dirname, "../local-prover");

    for (const stalePath of [
        path.join(webOutDir, "psy_prover.js"),
        path.join(webOutDir, "psy_prover.d.ts"),
        path.join(webOutDir, "psy_prover_bg.wasm"),
        path.join(webOutDir, "psy_prover_bg.wasm.d.ts"),
        path.join(nodeOutDir, "psy_prover.js"),
        path.join(nodeOutDir, "psy_prover.d.ts"),
        path.join(nodeOutDir, "psy_prover_bg.wasm"),
        path.join(nodeOutDir, "psy_prover_bg.wasm.d.ts"),
    ]) {
        if (fs.existsSync(stalePath)) {
            fs.rmSync(stalePath, { force: true });
        }
    }

    runWasmPack("web", webOutDir, rustSdkDir);
    runWasmPack("nodejs", nodeOutDir, rustSdkDir);

    const gitignoreTemplate = path.resolve(__dirname, "../../../../../.github/templates/.gitignore.wasm");
    if (fs.existsSync(gitignoreTemplate)) {
        fs.copyFileSync(gitignoreTemplate, path.join(webOutDir, ".gitignore"));
        fs.copyFileSync(gitignoreTemplate, path.join(nodeOutDir, ".gitignore"));
    }

    if (!fs.existsSync(wasmPath)) {
        throw new Error(`Expected wasm artifact at ${wasmPath}, but it was not generated`);
    }
}

ensureWasmArtifacts();

const wasmBuffer = fs.readFileSync(wasmPath);
const base64Chunks = wasmBuffer.toString("base64").match(/.{1,65536}/g) ?? [];
const base64ChunkString = base64Chunks.map((chunk) => `    ${JSON.stringify(chunk)},`).join("\n");

// Generate TypeScript module
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

// Write to output file
const outputPath = path.join(__dirname, './wasm-binary.ts');
fs.writeFileSync(outputPath, moduleContent);

console.log('WASM binary module generated successfully!');
console.log(`Size: ${wasmBuffer.length} bytes`);
