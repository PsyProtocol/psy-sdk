import { createHash } from "crypto";
import path from "path";
import { spawnSync } from "child_process";

export interface CompilerProvenance {
    compilerRevision: string;
    compilerSourcesHash: string;
}

interface CompilerSourceEntry {
    objectId: string;
    relativePath: string;
}

function runGit(psyCompilerDir: string, args: string[], input?: Buffer): Buffer {
    const result = spawnSync("git", args, {
        cwd: psyCompilerDir,
        input,
        maxBuffer: 64 * 1024 * 1024,
    });

    if (result.status !== 0) {
        const detail = result.stderr?.toString("utf8").trim();
        throw new Error(`git ${args.join(" ")} failed${detail ? `: ${detail}` : ""}`);
    }

    return result.stdout;
}

function parseNullDelimitedPaths(output: Buffer): string[] {
    return output
        .toString("utf8")
        .split("\0")
        .filter((relativePath) => relativePath.length > 0);
}

export function isCompilerSourcePath(relativePath: string): boolean {
    const normalized = relativePath.replace(/\\/g, "/");
    const lowerPath = normalized.toLowerCase();

    if (normalized === ".compiler-artifact.json") {
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

function readDirtyCompilerSourcePaths(psyCompilerDir: string): string[] {
    const changedPaths = [
        ...parseNullDelimitedPaths(runGit(psyCompilerDir, ["diff", "--name-only", "--no-renames", "-z", "HEAD", "--"])),
        ...parseNullDelimitedPaths(runGit(psyCompilerDir, ["diff", "--cached", "--name-only", "--no-renames", "-z", "HEAD", "--"])),
        ...parseNullDelimitedPaths(runGit(psyCompilerDir, ["ls-files", "--others", "--exclude-standard", "-z"])),
    ];

    return Array.from(new Set(changedPaths.filter(isCompilerSourcePath)))
        .sort((left, right) => Buffer.compare(Buffer.from(left, "utf8"), Buffer.from(right, "utf8")));
}

function assertCompilerSourcesClean(psyCompilerDir: string): void {
    const dirtySourcePaths = readDirtyCompilerSourcePaths(psyCompilerDir);
    if (dirtySourcePaths.length > 0) {
        throw new Error(
            `Compiler source tree has uncommitted eligible source changes:\n${dirtySourcePaths.map((sourcePath) => `- ${sourcePath}`).join("\n")}`,
        );
    }
}

function readCompilerSourceEntries(psyCompilerDir: string, compilerRevision: string): CompilerSourceEntry[] {
    return runGit(psyCompilerDir, ["ls-tree", "-r", "--full-tree", "-z", compilerRevision])
        .toString("utf8")
        .split("\0")
        .filter((entry) => entry.length > 0)
        .map((entry) => {
            const match = /^(\d+) (\w+) ([0-9a-f]+)\t([\s\S]+)$/.exec(entry);
            if (!match) {
                throw new Error(`Could not parse compiler source tree entry: ${entry}`);
            }

            return {
                objectType: match[2],
                objectId: match[3],
                relativePath: match[4],
            };
        })
        .filter((entry) => entry.objectType === "blob" && isCompilerSourcePath(entry.relativePath))
        .map(({ objectId, relativePath }) => ({ objectId, relativePath }))
        .sort((left, right) => Buffer.compare(Buffer.from(left.relativePath, "utf8"), Buffer.from(right.relativePath, "utf8")));
}

function readCompilerSourceBlobs(psyCompilerDir: string, entries: CompilerSourceEntry[]): Buffer[] {
    if (entries.length === 0) {
        return [];
    }

    const request = Buffer.from(`${entries.map(({ objectId }) => objectId).join("\n")}\n`, "ascii");
    const response = runGit(psyCompilerDir, ["cat-file", "--batch"], request);
    const sourceBlobs: Buffer[] = [];
    let offset = 0;

    for (const entry of entries) {
        const headerEnd = response.indexOf(0x0a, offset);
        if (headerEnd < 0) {
            throw new Error(`Missing git object header for ${entry.relativePath}`);
        }

        const header = response.subarray(offset, headerEnd).toString("ascii");
        const match = /^([0-9a-f]+) blob (\d+)$/.exec(header);
        if (!match || match[1] !== entry.objectId) {
            throw new Error(`Unexpected git object header for ${entry.relativePath}: ${header}`);
        }

        const size = Number(match[2]);
        const contentStart = headerEnd + 1;
        const contentEnd = contentStart + size;
        if (contentEnd >= response.length || response[contentEnd] !== 0x0a) {
            throw new Error(`Incomplete git object data for ${entry.relativePath}`);
        }

        sourceBlobs.push(response.subarray(contentStart, contentEnd));
        offset = contentEnd + 1;
    }

    if (offset !== response.length) {
        throw new Error("Unexpected trailing data while reading compiler source objects");
    }

    return sourceBlobs;
}

export function readCompilerProvenance(psyCompilerDir: string): CompilerProvenance {
    const compilerRevision = runGit(psyCompilerDir, ["rev-parse", "HEAD"]).toString("utf8").trim();
    assertCompilerSourcesClean(psyCompilerDir);

    const sourceEntries = readCompilerSourceEntries(psyCompilerDir, compilerRevision);
    const sourceBlobs = readCompilerSourceBlobs(psyCompilerDir, sourceEntries);
    const hash = createHash("sha256");
    const separator = Buffer.from([0]);

    sourceEntries.forEach(({ relativePath }, index) => {
        hash.update(Buffer.from(relativePath, "utf8"));
        hash.update(separator);
        hash.update(sourceBlobs[index]);
        hash.update(separator);
    });

    return {
        compilerRevision,
        compilerSourcesHash: hash.digest("hex"),
    };
}
