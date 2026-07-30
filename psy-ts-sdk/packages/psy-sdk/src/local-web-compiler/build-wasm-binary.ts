import fs from "fs";
import { createHash } from "crypto";
import path from "path";
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const wasmPath = path.join(__dirname, "./psy_compiler_bg.wasm");
const compilerArtifactPath = path.resolve(__dirname, "../../.compiler-artifact.json");

function runGit(psyCompilerDir: string, args: string[]): string {
    const result = spawnSync("git", args, {
        cwd: psyCompilerDir,
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
    });

    if (result.status !== 0) {
        const detail = result.stderr?.trim();
        throw new Error(`git ${args.join(" ")} failed${detail ? `: ${detail}` : ""}`);
    }

    return result.stdout;
}

function isCompilerSourcePath(relativePath: string): boolean {
    const normalized = relativePath.replace(/\\/g, "/");
    const lowerPath = normalized.toLowerCase();
    const basename = path.posix.basename(lowerPath);
    const containsSensitiveName = ["credential", "credentials", "secret", "secrets", "auth"]
        .some((name) => lowerPath.includes(name));

    if (
        basename.startsWith(".env")
        || containsSensitiveName
        || lowerPath.endsWith(".pem")
        || lowerPath.endsWith(".key")
        || normalized === ".compiler-artifact.json"
    ) {
        return false;
    }

    if (["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "Makefile"].includes(normalized)) {
        return true;
    }

    if (["build.rs", "precompiles.json", "package.json"].includes(path.posix.basename(normalized))) {
        return true;
    }

    return [".rs", ".psy", ".toml", ".lock"].some((extension) => lowerPath.endsWith(extension));
}

function readCompilerProvenance(psyCompilerDir: string): {
    compilerRevision: string;
    compilerSourcesHash: string;
} {
    const compilerRevision = runGit(psyCompilerDir, ["rev-parse", "HEAD"]).trim();
    const sourcePaths = runGit(psyCompilerDir, ["ls-files", "--cached", "--others", "--exclude-standard"])
        .split(/\r?\n/)
        .filter((relativePath) => relativePath.length > 0 && isCompilerSourcePath(relativePath))
        .sort((left, right) => Buffer.compare(Buffer.from(left, "utf8"), Buffer.from(right, "utf8")));
    const hash = createHash("sha256");
    const separator = Buffer.from([0]);

    for (const relativePath of sourcePaths) {
        const sourcePath = path.resolve(psyCompilerDir, relativePath);
        const stat = fs.lstatSync(sourcePath);
        let sourceBytes: Buffer;
        if (stat.isFile()) {
            sourceBytes = fs.readFileSync(sourcePath);
        } else if (stat.isSymbolicLink()) {
            sourceBytes = Buffer.from(fs.readlinkSync(sourcePath), "utf8");
        } else {
            continue;
        }
        hash.update(Buffer.from(relativePath, "utf8"));
        hash.update(separator);
        hash.update(sourceBytes);
        hash.update(separator);
    }

    return {
        compilerRevision,
        compilerSourcesHash: hash.digest("hex"),
    };
}

function writeCompilerArtifact(psyCompilerDir: string): void {
    const artifact = readCompilerProvenance(psyCompilerDir);
    const temporaryPath = `${compilerArtifactPath}.tmp`;
    fs.writeFileSync(temporaryPath, `${JSON.stringify(artifact, null, 2)}\n`);
    fs.renameSync(temporaryPath, compilerArtifactPath);
}

function runWasmPack(outDir: string, psyCompilerDir: string) {
    const wasmCrateDir = "psy-wasm";
    const manifestPath = path.join(psyCompilerDir, wasmCrateDir, "Cargo.toml");
    if (!fs.existsSync(manifestPath)) {
        throw new Error(`Standalone compiler manifest not found at ${manifestPath}`);
    }
    const args = ["build", wasmCrateDir, "--target", "web", "--out-dir", outDir, "--out-name", "psy_compiler", "--no-pack", "--release"];
    const result = spawnSync("wasm-pack", args, {
        cwd: psyCompilerDir,
        stdio: "inherit",
        env: process.env,
    });

    if (result.status !== 0) {
        throw new Error("wasm-pack build failed for standalone compiler target \"web\"");
    }
}

function ensureWasmArtifacts(): string | undefined {
    const envCompilerDir = process.env.PSY_COMPILER_DIR?.trim();
    const siblingCompilerDir = path.resolve(__dirname, "../../../../../../psy-compiler");
    const psyCompilerDir = envCompilerDir ? path.resolve(envCompilerDir) : siblingCompilerDir;

    if (process.env.SKIP_WASM_PACK) {
        console.log("SKIP_WASM_PACK set: re-encoding existing compiler WASM, skipping wasm-pack...");
        if (!fs.existsSync(wasmPath)) {
            throw new Error(`SKIP_WASM_PACK set but no wasm artifact found at ${wasmPath}`);
        }
        fs.rmSync(compilerArtifactPath, { force: true });
        return undefined;
    }

    console.log("Building compiler WASM artifacts via wasm-pack...");

    const webOutDir = path.resolve(__dirname);
    fs.rmSync(compilerArtifactPath, { force: true });
    runWasmPack(webOutDir, psyCompilerDir);

    const gitignoreTemplate = path.resolve(__dirname, "../../../../../.github/templates/.gitignore.wasm");
    if (fs.existsSync(gitignoreTemplate)) {
        fs.copyFileSync(gitignoreTemplate, path.join(webOutDir, ".gitignore"));
    }

    if (!fs.existsSync(wasmPath)) {
        throw new Error(`Expected wasm artifact at ${wasmPath}, but it was not generated`);
    }

    return psyCompilerDir;
}

const builtCompilerDir = ensureWasmArtifacts();

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

if (builtCompilerDir) {
    writeCompilerArtifact(builtCompilerDir);
}

console.log("Compiler WASM binary module generated successfully!");
console.log(`Size: ${wasmBuffer.length} bytes`);
