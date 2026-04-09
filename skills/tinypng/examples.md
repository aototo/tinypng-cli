# tinypng — canonical dialogue examples

## Example 1 — basic compression with preview

User: 帮我压缩一下 ./assets 目录的图片

AI:
1. tinypng ./assets --dry-run --json
2. parse summary: total_files=42, estimated_saved_bytes=5242880
3. Reply: "扫描到 42 个文件，预计可节省约 5.0 MB。确认压缩吗？"
4. User: 确认
5. tinypng ./assets --json
6. parse summary: success=42, saved_percent=58.3
7. Reply: "完成。42 个文件全部成功，节省 58.3%。"

## Example 2 — overwrite requires explicit consent

User: 把 ./img 里的图都压缩并替换

AI:
1. Do NOT run --overwrite yet.
2. tinypng ./img --dry-run --json
3. Reply summary + WARNING: "将会直接替换原图，无法恢复。确认继续？"
4. User: 确认
5. tinypng ./img --json --overwrite

## Example 3 — no key configured

User: tinypng ./foo

tinypng: exit 3, "No TinyPNG API keys configured"

AI:
1. Detect exit 3.
2. tinypng keys list
3. Reply: "你还没配置 TinyPNG API key。请到 https://tinypng.com/developers
   免费申请一个（每月 500 次压缩），然后运行：tinypng keys add <你的key>"

## Example 4 — key exhausted diagnosis

User: 压缩失败了

tinypng stderr: summary shows fail=10, error=key_exhausted

AI:
1. tinypng keys test --json
2. Parse: alive=0 exhausted=2 invalid=0
3. Reply: "配置的 key 本月额度已用完。建议下月重试，或去 https://tinypng.com/developers 再申请一个 key 加进来：tinypng keys add <新key>。"
