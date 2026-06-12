# OpenClaw / 小龙虾 接入

## MCP

若 Agent 支持 MCP，配置同 [nanobot](nanobot.md)：

```json
{
  "command": "dayrecord",
  "args": ["mcp"]
}
```

## Shell / 工具调用

在 Agent 工具中执行：

```bash
dayrecord context --scope user --format json
dayrecord context --scope recent:7 --format md
```

## 文件导出

```bash
dayrecord export --target openclaw
```

产出 `USER.md`、`MEMORY.md` 与 `workspace/dayrecord-*.md`，可软链到 OpenClaw workspace 记忆目录。
