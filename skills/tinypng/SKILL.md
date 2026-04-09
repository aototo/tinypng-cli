---
name: tinypng
description: Use when user wants to compress images (PNG/JPG/WebP), reduce image size, batch optimize assets, or mentions TinyPNG / 压缩图片 / 图片优化 / 减小图片. Triggers include "压缩这张图", "把这个目录的图都压一下", "帮我优化一下 assets 目录", "shrink this", "compress these images", "reduce image size".
---

# tinypng - Image Compression CLI

`tinypng` is a Rust CLI that compresses images using the TinyPNG API with a key pool for high throughput.

## When to Use

- User wants to reduce image file size
- User mentions "压缩图片" / "optimize images" / "tinypng" / "reduce size"
- User provides a path and asks to make it smaller

## When NOT to Use

- User wants to resize/crop (use imagemagick)
- User wants format conversion (e.g., PNG → WebP)
- User wants lossless-only compression (tinypng is lossy via TinyPNG API)

## Core Workflow

Always follow this decision tree.

### Step 1: Preview with --dry-run

ALWAYS run `--dry-run --json` first to see scope:

    tinypng <path> --dry-run --json

Read the NDJSON output:
- `start` event → total files and bytes
- `file` events with `status: "dry_run"` → per-file preview
- `summary` event → total estimated savings

Report the preview to the user and ask for confirmation before the real run,
UNLESS the user already said "just do it" / "直接压" / explicitly skipped preview.

### Step 2: Real run

    tinypng <path> --json

If user wants progress visibility, omit `--json` to show the progress bar.

### Step 3: Handle exit code

| Exit | Meaning       | Action                                                     |
|------|---------------|------------------------------------------------------------|
| 0    | All success   | Report saved_percent from summary                           |
| 1    | Partial fail  | Parse JSON, group failures by `error` field, report        |
| 2    | Bad args      | Fix the command, retry                                     |
| 3    | Config error  | Run `tinypng keys list` → likely `no_keys_configured`      |
| 4    | All failed    | Run `tinypng keys test` → check network or exhausted pool  |

## Error Code Reference

See error-handling.md for the full table. Key points:

- `key_exhausted` (any file): run `tinypng keys test` to see which keys are dead
- `no_keys_configured`: run `tinypng keys add <key>`, guide user to https://tinypng.com/developers
- `invalid_image`: skip, report to user, do NOT retry
- `file_too_large`: >5MB; suggest resize first
- `network_timeout` / `server_error`: retry once after 2s

## Output Parsing Rules

`--json` emits NDJSON (one JSON object per line). Parse line by line:

    type == "start"    → extract total_files, total_bytes
    type == "file"     → check status (success/fail/skipped/dry_run)
    type == "progress" → ignore unless user asked for progress
    type == "summary"  → final stats; exit_code is here

NEVER parse the human-readable output. ALWAYS use --json for programmatic decisions.

## Common Commands

    tinypng <path>                     compress in-place (adds compressed_ prefix)
    tinypng <path> --overwrite         compress and replace originals
    tinypng <path> -o ./out            compress to output dir
    tinypng <path> --dry-run           preview, no changes
    tinypng <path> --json              AI-friendly output
    tinypng <path> -c 8                higher concurrency
    tinypng keys list                  show configured keys (hashes only)
    tinypng keys test                  probe each key for health/quota
    tinypng keys add <key>             add a key to config
    tinypng config get                 show all settings

## Safety Rules

- NEVER run `--overwrite` without user confirmation, even if user said "compress this dir".
- NEVER show the full key value when reporting key errors; use the 8-char hash from JSON.
- If `tinypng keys test` shows any key as `invalid`, suggest `tinypng keys remove <hash>` to clean up.
- If more than 50% of keys are `exhausted`, warn user about monthly quota.
- If user has no key configured (exit 3), guide them to https://tinypng.com/developers to get a free one (500 compressions/month).
