# tinypng — command quick reference

## Main command

    tinypng <PATHS...> [options]

### Options

| Flag | Default | Effect |
|------|---------|--------|
| `-o, --output <DIR>` | in-place | Write compressed files here |
| `-w, --overwrite` | false | Replace originals |
| `-e, --ext <LIST>` | png,jpg,jpeg,webp | Extensions to process |
| `--min-size <SIZE>` | 10k | Skip files smaller than this |
| `--no-skip-compressed` | false | Also process `compressed_*` files |
| `-c, --concurrency <N>` | 4 | Parallel workers |
| `-n, --dry-run` | false | Scan only, no changes |
| `--json` | false | NDJSON machine output |
| `-q, --quiet` | false | Only show summary |
| `-v, --verbose` | false | Log key rotation, retries |

## keys subcommand

    tinypng keys list [--json]
    tinypng keys test [--json] [-c N]
    tinypng keys add <KEY>
    tinypng keys remove <KEY_OR_HASH_PREFIX>

## config subcommand

    tinypng config get [KEY]
    tinypng config set <KEY> <VALUE>
    tinypng config path
    tinypng config edit
