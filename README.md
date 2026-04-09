# tinypng-cli

[![CI](https://github.com/aototo/tinypng-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/aototo/tinypng-cli/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tinypng-cli.svg)](https://crates.io/crates/tinypng-cli)
[![npm](https://img.shields.io/npm/v/tinypng-cli.svg)](https://www.npmjs.com/package/tinypng-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Fast batch image compression powered by [TinyPNG](https://tinypng.com/), built for humans and AI.

A single-binary CLI that batch-compresses PNG / JPG / WebP, parallelized across a pool of TinyPNG API keys, with both human-readable and NDJSON output modes — so both you and your AI assistant can drive it.

## Features

- **Fast** — parallel workers, automatic key rotation, concurrent requests
- **Safe** — writes to `compressed_*` by default; `--overwrite` is opt-in
- **AI-friendly** — stable NDJSON output (`--json`) with stable error codes
- **Dry-run first** — preview before you burn API quota (`--dry-run`)
- **Multi-key pool** — bring multiple TinyPNG keys for higher throughput; auto-rotates on quota / failure
- **Ships a Claude Code Skill** — drop it in and say "压缩 ./assets"

## Install

### Option 1 — via npm (works for everyone, no Rust needed)

```bash
npm install -g tinypng-cli
```

The npm package is a thin wrapper that auto-downloads the matching prebuilt binary from GitHub Releases for your platform (macOS / Linux / Windows, x64 / arm64).

### Option 2 — via Cargo (Rust users)

```bash
cargo install tinypng-cli
```

### Option 3 — curl one-liner (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/aototo/tinypng-cli/main/install.sh | bash
```

Downloads the right tarball from GitHub Releases and installs the `tinypng` binary to `~/.local/bin`.

### Option 4 — download a release manually

Grab the archive for your platform from [GitHub Releases](https://github.com/aototo/tinypng-cli/releases), extract it, and move `tinypng` somewhere on your `$PATH`.

## Configure a TinyPNG API key

`tinypng-cli` does not ship with any API keys. You need your own (free).

1. Sign up at <https://tinypng.com/developers> — free tier gives you **500 compressions per month per key**.
2. Register your key:

```bash
tinypng keys add <your-key>
```

Or via environment variable (useful for CI):

```bash
export TINIFY_KEY=<your-key>
```

Or multiple keys at once (merged into one pool, rotated automatically):

```bash
export TINIFY_KEYS=key1,key2,key3
```

Verify:

```bash
tinypng keys list
tinypng keys test
```

## Usage

```bash
# Preview — uses zero quota
tinypng ./assets --dry-run

# Compress in place (writes compressed_xxx alongside)
tinypng ./assets

# Replace originals (be sure — no undo)
tinypng ./assets --overwrite

# Output to a different directory
tinypng ./assets -o ./compressed

# NDJSON output for scripts / AI
tinypng ./assets --json

# Higher concurrency
tinypng ./assets -c 8
```

`tinypng --help` for everything else.

### Exit codes

| Code | Meaning            |
|------|--------------------|
| 0    | All success        |
| 1    | Partial failure    |
| 2    | Bad arguments      |
| 3    | No keys configured |
| 4    | All keys failed    |

## Use with Claude Code

This repo ships a Claude Code Skill at `skills/tinypng/`. In Claude Code, just say it in Chinese or English:

> 帮我压缩 ~/Downloads/某目录 里的图片

Claude will run `tinypng --dry-run --json` first, show you the plan, and only proceed on confirmation. See `skills/tinypng/SKILL.md` for details.

## Configuration file

`tinypng-cli` reads `~/.config/tinypng/config.toml` (if present). Example:

```toml
concurrency = 8
overwrite = false
min_size = "20k"
extensions = ["png", "jpg", "jpeg", "webp"]

[keys]
values = ["your-tinypng-key-1", "your-tinypng-key-2"]
```

CLI flags take precedence over the config file, which takes precedence over built-in defaults.

## Development

```bash
make build     # cargo build --release
make test      # cargo test
make clean     # cargo clean
```

Or directly with Cargo:

```bash
cargo build
cargo test
cargo run -- ./some-dir --dry-run
```

Releases are automated via GitHub Actions — push a tag `v0.X.Y` and the workflow builds binaries for 5 platforms, uploads them to the Release, and publishes to crates.io and npm.

## License

[MIT](LICENSE) © aototo
