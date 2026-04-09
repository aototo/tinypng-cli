# tinypng — error handling reference

## Error code table

All values in the `error` field of `status: "fail"` file events are stable
snake_case strings. AI should switch on them directly.

| code | cause | action |
|------|-------|--------|
| `invalid_image` | TinyPNG rejected file as non-image | skip, tell user |
| `unsupported_format` | Extension not supported | check extension, skip |
| `file_too_large` | >5 MB | resize first, then retry |
| `key_exhausted` | Runner-level: all keys hit quota | `tinypng keys test`, wait or add keys |
| `key_invalid` | Runner-level: all keys rejected | verify config |
| `no_keys_configured` | No keys in env/config | `tinypng keys add <key>` |
| `all_keys_exhausted` | No healthy keys at start | same as above two |
| `network_timeout` | DNS/connect/read timeout | retry after 2s |
| `server_error` | TinyPNG 5xx | retry after 5s |
| `io_error` | Local fs read/write failure | check disk/permissions |
| `permission_denied` | Write denied | check target dir perms |
| `max_retries_exceeded` | File failed after 5 key switches | file is bad, skip |
| `config_parse` | TOML syntax error | `tinypng config edit` |
| `bad_argument` | Invalid flag combo or missing path | fix command |
| `protocol_error` | Unexpected TinyPNG response | retry once, then report |

## Diagnosis flow

Exit 3:
  1. `tinypng keys list` → sum is 0?
  2. yes → `tinypng keys add <key>` flow
  3. no → check `~/.config/tinypng/config.toml` syntax

Exit 4:
  1. `tinypng keys test`
  2. Count exhausted vs invalid
  3. If all exhausted → wait for monthly reset or add more keys
  4. If network_error → check connectivity
