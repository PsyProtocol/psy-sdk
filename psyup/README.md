# psyup

The QED toolchain installer and manager, inspired by [foundryup](https://github.com/foundry-rs/foundry/tree/master/foundryup).

## Installation

To install psyup, run:

```bash
curl -L https://raw.githubusercontent.com/QEDProtocol/qedlang-rust/master/psyup/install | bash
```

Then restart your shell or run `source ~/.bashrc` (or appropriate shell config).

## Usage

### Install Latest Stable Version

```bash
psyup
```

### Install Specific Version

```bash
psyup --install v1.0.0
```

### Install Nightly

```bash
psyup --install nightly
```

### Build from Source (Latest)

```bash
psyup --repo QEDProtocol/qedlang-rust
```

### Build from Local Repository

```bash
psyup --path /path/to/qedlang-rust
```

### List Installed Versions

```bash
psyup --list
```

### Switch Between Versions

```bash
psyup --use v1.0.0
```

### Update psyup Itself

```bash
psyup --update
```

## What Gets Installed

psyup installs the following QED tools:

- **qed_user_cli** - User-facing command line interface
- **qed_rollup_cli** - Rollup management and operation tools
- **qed_dev_cli** - Development and testing utilities
- **dargo** - QED language compiler and toolchain
- **psy-lsp-server** - Language Server Protocol support for IDEs

## Directory Structure

psyup installs everything under `~/.qed/`:

```
~/.qed/
├── bin/           # Symlinks to current version binaries
├── versions/      # Version-specific installations
│   ├── stable/
│   ├── v1.0.0/
│   └── nightly/
└── share/man/     # Man pages
```

## Multi-Platform Support

psyup supports the same platforms as the main QED toolchain:

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)
- Alpine Linux (musl variants)

## Options

- `-h, --help` - Print help information
- `-v, --version` - Print psyup version
- `-U, --update` - Update psyup itself
- `-i, --install <VERSION>` - Install specific version
- `-l, --list` - List installed versions
- `-u, --use <VERSION>` - Use specific version
- `-b, --branch <BRANCH>` - Build from specific branch
- `-P, --pr <PR>` - Build from pull request
- `-C, --commit <COMMIT>` - Build from specific commit
- `-r, --repo <REPO>` - Build from GitHub repository
- `-p, --path <PATH>` - Build from local repository
- `-j, --jobs <N>` - Number of build jobs
- `--arch <ARCH>` - Target architecture (amd64, arm64)
- `--platform <PLATFORM>` - Target platform (linux, darwin, win32, alpine)

## Examples

```bash
# Install latest stable
psyup

# Install specific version
psyup --install v1.2.0

# Install from main branch
psyup --branch main

# Install from pull request
psyup --pr 123

# Build from local source
psyup --path ~/qedlang-rust

# Use specific installed version
psyup --use v1.1.0

# List all installed versions
psyup --list
```