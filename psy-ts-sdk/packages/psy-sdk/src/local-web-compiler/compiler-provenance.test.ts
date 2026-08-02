import { createHash } from "crypto";
import fs from "fs";
import os from "os";
import path from "path";
import { spawnSync } from "child_process";
import { readCompilerProvenance } from "./compiler-provenance";

function runGit(repositoryDir: string, args: string[]): string {
    const result = spawnSync("git", args, {
        cwd: repositoryDir,
        encoding: "utf8",
        env: {
            ...process.env,
            GIT_AUTHOR_EMAIL: "compiler-provenance@example.invalid",
            GIT_AUTHOR_NAME: "Compiler Provenance Test",
            GIT_COMMITTER_EMAIL: "compiler-provenance@example.invalid",
            GIT_COMMITTER_NAME: "Compiler Provenance Test",
        },
    });

    if (result.status !== 0) {
        throw new Error(result.stderr.trim());
    }

    return result.stdout.trim();
}

function writeSource(repositoryDir: string, relativePath: string, contents: string): void {
    const sourcePath = path.join(repositoryDir, relativePath);
    fs.mkdirSync(path.dirname(sourcePath), { recursive: true });
    fs.writeFileSync(sourcePath, contents);
}

function hashSources(sources: Array<[string, string]>): string {
    const hash = createHash("sha256");
    const separator = Buffer.from([0]);

    sources
        .sort(([left], [right]) => Buffer.compare(Buffer.from(left, "utf8"), Buffer.from(right, "utf8")))
        .forEach(([relativePath, contents]) => {
            hash.update(Buffer.from(relativePath, "utf8"));
            hash.update(separator);
            hash.update(Buffer.from(contents, "utf8"));
            hash.update(separator);
        });

    return hash.digest("hex");
}

describe("compiler artifact provenance", () => {
    let repositoryDir: string;

    beforeEach(() => {
        repositoryDir = fs.mkdtempSync(path.join(os.tmpdir(), "psy-compiler-provenance-"));
        runGit(repositoryDir, ["init", "--quiet"]);
    });

    afterEach(() => {
        fs.rmSync(repositoryDir, { recursive: true, force: true });
    });

    it("hashes eligible compiler sources from the tracked HEAD blobs", () => {
        const cargoToml = "[package]\nname = \"fixture\"\n";
        const mainRust = "fn main() {}\n";
        writeSource(repositoryDir, "Cargo.toml", cargoToml);
        writeSource(repositoryDir, "src/main.rs", mainRust);
        writeSource(repositoryDir, "README.md", "fixture\n");
        runGit(repositoryDir, ["add", "."]);
        runGit(repositoryDir, ["commit", "--quiet", "-m", "fixture"]);

        writeSource(repositoryDir, "README.md", "uncommitted documentation change\n");

        expect(readCompilerProvenance(repositoryDir)).toEqual({
            compilerRevision: runGit(repositoryDir, ["rev-parse", "HEAD"]),
            compilerSourcesHash: hashSources([
                ["Cargo.toml", cargoToml],
                ["src/main.rs", mainRust],
            ]),
        });
    });

    it("rejects tracked eligible source changes", () => {
        writeSource(repositoryDir, "src/main.rs", "fn main() {}\n");
        runGit(repositoryDir, ["add", "."]);
        runGit(repositoryDir, ["commit", "--quiet", "-m", "fixture"]);
        writeSource(repositoryDir, "src/main.rs", "fn main() { panic!(); }\n");

        expect(() => readCompilerProvenance(repositoryDir)).toThrow(
            "Compiler source tree has uncommitted eligible source changes:\n- src/main.rs",
        );
    });

    it("rejects staged eligible source changes", () => {
        writeSource(repositoryDir, "src/main.rs", "fn main() {}\n");
        runGit(repositoryDir, ["add", "."]);
        runGit(repositoryDir, ["commit", "--quiet", "-m", "fixture"]);
        writeSource(repositoryDir, "src/main.rs", "fn main() { panic!(); }\n");
        runGit(repositoryDir, ["add", "src/main.rs"]);

        expect(() => readCompilerProvenance(repositoryDir)).toThrow(
            "Compiler source tree has uncommitted eligible source changes:\n- src/main.rs",
        );
    });

    it("rejects untracked eligible source changes", () => {
        writeSource(repositoryDir, "Cargo.toml", "[workspace]\n");
        runGit(repositoryDir, ["add", "."]);
        runGit(repositoryDir, ["commit", "--quiet", "-m", "fixture"]);
        writeSource(repositoryDir, "src/untracked.rs", "pub fn untracked() {}\n");

        expect(() => readCompilerProvenance(repositoryDir)).toThrow(
            "Compiler source tree has uncommitted eligible source changes:\n- src/untracked.rs",
        );
    });

    it("keeps the tracked artifact synchronized with the clean compiler HEAD", () => {
        const compilerDir = path.resolve(process.env.PSY_COMPILER_DIR ?? path.join(process.cwd(), "../../../../psy-compiler"));
        const artifact = JSON.parse(fs.readFileSync(path.join(process.cwd(), ".compiler-artifact.json"), "utf8"));

        expect(readCompilerProvenance(compilerDir)).toEqual(artifact);
    });
});
