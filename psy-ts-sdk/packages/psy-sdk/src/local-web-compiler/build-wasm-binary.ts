import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_compiler_bg.wasm");

function runWasmPack(target: "web" | "nodejs", outDir: string, psyCompilerWorkspace: string, cratePath: string) {
    const args = ["build", cratePath, "--target", target, "--out-dir", outDir, "--out-name", "psy_compiler", "--no-pack", "--release"];
    const result = spawnSync("wasm-pack", args, {
        cwd: psyCompilerWorkspace,
        stdio: "inherit",
        env: process.env,
    });

    if (result.status !== 0) {
        throw new Error(`wasm-pack build failed for target "${target}"`);
    }
}

function ensureWasmArtifacts() {
    console.log("Building compiler WASM artifacts via wasm-pack...");

    const envCompilerDir = process.env.PSY_COMPILER_WASM_WORKSPACE?.trim() || process.env.PSY_COMPILER_DIR?.trim();
    const defaultParthCompilerWorkspace = path.resolve(
        __dirname,
        "../../../../../../parth-generic-v1/client_prover/psy_ide",
    );
    const legacyCompilerDir = path.resolve(__dirname, "../../../../../../psy-compiler");
    const candidates = [
        envCompilerDir ? path.resolve(envCompilerDir) : null,
        defaultParthCompilerWorkspace,
        legacyCompilerDir,
    ].filter((candidate): candidate is string => Boolean(candidate));
    const compiler = candidates.flatMap((workspace) => [
        { workspace, cratePath: "psy_wasm", cargoToml: path.join(workspace, "psy_wasm", "Cargo.toml") },
        { workspace, cratePath: "psy-wasm", cargoToml: path.join(workspace, "psy-wasm", "Cargo.toml") },
        { workspace, cratePath: ".", cargoToml: path.join(workspace, "Cargo.toml") },
    ]).find((candidate) => fs.existsSync(candidate.cargoToml));
    if (!compiler) {
        throw new Error(`Could not find compiler WASM crate. Checked: ${candidates.join(", ")}`);
    }
    const webOutDir = path.resolve(__dirname);

    runWasmPack("web", webOutDir, compiler.workspace, compiler.cratePath);

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
