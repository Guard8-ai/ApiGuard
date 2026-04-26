# Installation

## Pre-built Binaries (recommended)

Download the latest release for your platform from the [releases page](https://github.com/Guard8-ai/ApiGuard/releases):

| Platform | File |
|----------|------|
| Linux x86_64 | `apiguard-linux-x86_64` |
| macOS ARM64 | `apiguard-macos-aarch64` |
| Windows x86_64 | `apiguard-windows-x86_64.exe` |

```bash
# Linux / macOS
chmod +x apiguard-linux-x86_64
mv apiguard-linux-x86_64 ~/.local/bin/apiguard

# Verify
apiguard --version
```

## From Source

Requires Rust 1.80+.

```bash
git clone https://github.com/Guard8-ai/ApiGuard
cd ApiGuard
cargo install --path .
```

## Verify Installation

```bash
apiguard --version
apiguard --help
```
