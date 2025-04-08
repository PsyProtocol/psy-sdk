# Before We Begin

[QED Smart Contract Language] requires a development environment to write and run programs. This chapter covers the prerequisites: setting up your IDE, installing the compiler, and understanding the basic tools.

If you already have the compiler installed (via `git` or another method), you can skip to the next chapter.

## Installing the Compiler

Download the latest compiler binary from [https://github.com/QEDProtocol/qed-lang]. Binaries are available for macOS, Linux, and Windows.

### Using a Package Manager
- **Homebrew (macOS):** `brew install [dargo]`
- **Chocolatey (Windows):** `choco install [dargo]`
- **Cargo (Rust-based, if applicable):** `cargo install --git https://github.com/QEDProtocol/qed-lang dargo`

## Setting up environment

```fish
set -gx DARGO_STD_PATH "/path-to-qed-lang/qed-std/std.qed"
```

## Setting Up Your IDE

Recommended IDEs:
- **VSCode**: Install the [QED Smart Contract Language] extension for syntax highlighting.
- **IntelliJ IDEA**: Use the [QED Smart Contract Language] plugin by [QEDProtocol].
