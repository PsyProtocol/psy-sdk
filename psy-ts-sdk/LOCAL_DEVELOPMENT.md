# Local Development Setup

This guide covers how to build and use `psy-sdk` with local dependencies (compiler, prover, downstream consumers).

## Workspace packages

| Package | Name | Purpose |
|---|---|---|
| `packages/psy-sdk` | `@psy-protocol/psy-sdk` | Main SDK: RPC clients, wallet provider, local web prover, local web compiler WASM |
| `packages/contract-sdk` | `@psy-protocol/contract-sdk` | Contract codegen + runtime (`AbiConverter`, `Contract`, `SDKGenerator`) |
| `packages/utils` | `@psy-protocol/utils` | Shared utilities (UUID, JSON helpers) |

## WASM binary sources

The WASM binaries (`psy_compiler_bg.wasm`, `psy_prover_bg.wasm`) are built from Rust crates in sibling repos.

### Prover WASM (`psy_prover`)

Built from `parth-generic-v1/client_prover/psy_ide/psy_wasm` (vendored) or the standalone `psy-compiler/psy-wasm`.

```bash
# Default: uses vendored parth-generic-v1 path
pnpm --filter @psy-protocol/psy-sdk run build:wasm

# Or specify a custom path:
PSY_COMPILER_DIR=$HOME/Projects/psy-compiler \
  pnpm --filter @psy-protocol/psy-sdk run build:wasm
```

The build script (`packages/psy-sdk/src/local-web-prover/build-wasm-binary.ts`) looks for the Rust crate in this order:
1. `PSY_COMPILER_DIR` env var (absolute path)
2. `../../psy-compiler` (standalone compiler repo)
3. `../../parth-generic-v1/client_prover/psy_ide` (vendored in monorepo)

### Compiler WASM (`psy_compiler`)

Built from the same crate directory.

```bash
# Default: uses vendored path
pnpm --filter @psy-protocol/psy-sdk run build:wasm-compiler

# Or with custom path:
PSY_COMPILER_DIR=$HOME/Projects/psy-compiler \
  pnpm --filter @psy-protocol/psy-sdk run build:wasm-compiler
```

### Full SDK build

```bash
# Build WASM binaries + TypeScript (from local sources):
pnpm --filter @psy-protocol/psy-sdk run build

# Then rebuild contract-sdk:
pnpm --filter @psy-protocol/contract-sdk run build

# Or build everything:
pnpm run build
```

## Using local psy-sdk in downstream repos

### psy-wallet

```bash
# Option 1: file: dependency in psy-wallet/package.json
"@psy-protocol/psy-sdk": "file:../psy-sdk/psy-ts-sdk/packages/psy-sdk",
"@psy-protocol/contract-sdk": "file:../psy-sdk/psy-ts-sdk/packages/contract-sdk",

# Option 2: pnpm link
cd $HOME/Projects/psy-sdk/psy-ts-sdk/packages/psy-sdk
pnpm link --global

cd $HOME/Projects/psy-wallet
pnpm link --global @psy-protocol/psy-sdk
pnpm run build:dev
```

### psy-explorer (in parth-generic-v1)

```bash
# In parth-generic-v1/client_prover/psy_explorer/package.json:
"@psy-protocol/psy-sdk": "file:../../psy-sdk/psy-ts-sdk/packages/psy-sdk",
"@psy-protocol/contract-sdk": "file:../../psy-sdk/psy-ts-sdk/packages/contract-sdk",
```

Or use pnpm link as above, then `pnpm install` in the explorer directory.

### psy-ide (in parth-generic-v1)

```bash
# In parth-generic-v1/client_prover/psy_ide/frontend/package.json:
"@psy-protocol/psy-sdk": "file:../../../../psy-sdk/psy-ts-sdk/packages/psy-sdk",
"@psy-protocol/contract-sdk": "file:../../../../psy-sdk/psy-ts-sdk/packages/contract-sdk",
```

## Quick local development workflow

```bash
# 1. Rebuild WASM after compiler changes:
cd $HOME/Projects/psy-sdk/psy-ts-sdk
PSY_COMPILER_DIR=$HOME/Projects/psy-compiler \
  pnpm --filter @psy-protocol/psy-sdk run build:wasm

# 2. Full psy-sdk build:
pnpm --filter @psy-protocol/psy-sdk run build

# 3. Rebuild contract-sdk:
pnpm --filter @psy-protocol/contract-sdk run build

# 4. Rebuild downstream:
cd $HOME/Projects/psy-wallet && pnpm run build:dev
# or
cd $HOME/Projects/parth-generic-v1/client_prover/psy_explorer && pnpm run build
cd $HOME/Projects/parth-generic-v1/client_prover/psy_ide/frontend && bun run build
```

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `PSY_COMPILER_DIR` | (auto-detected) | Override path to the compiler crate directory |

## Common pitfalls

1. **Stale WASM**: After changing compiler source, you must rebuild WASM (`build:wasm` / `build:wasm-compiler`). The `.wasm` files are git-ignored and not recompiled automatically.
2. **pnpm install after link**: After `pnpm link --global @psy-protocol/psy-sdk`, downstream repos need `pnpm install` to resolve the link.
3. **TypeScript version mismatch**: psy-sdk uses `typescript ~4.9.5`, contract-sdk uses `~4.9.5`, utils uses `~5.3.3`. If you see type errors across packages, ensure the right TS version is active per workspace.
4. **WASM not found**: If `import { wasmBinary } from "./wasm-binary"` fails, run `pnpm --filter @psy-protocol/psy-sdk run build:wasm` first — the `wasm-binary.ts` file is generated at build time and is git-ignored.