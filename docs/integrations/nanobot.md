# nanobot 接入

## MCP（推荐）

在 nanobot MCP 配置中添加：

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "dayrecord",
      "args": ["mcp"]
    }
  }
}
```

安装 CLI：

```bash
cargo install --path crates/dayrecord-cli
```

## 工具

| 工具 | 说明 |
|------|------|
| `get_user_profile` | 习惯画像 + 活跃事实 JSON |
| `query_user_facts` | 关键词搜索事实 |
| `get_recent_summary` | 近 N 日复盘 Markdown |

## 资源（MCP resources）

| URI | 说明 |
|-----|------|
| `dayrecord://user/profile` | 用户画像 + 活跃事实 JSON |
| `dayrecord://facts/active` | 全部活跃事实 JSON |
| `dayrecord://memory/{date}` | 指定日期复盘 Markdown（如 `2026-06-10`） |

## 文件兜底

```bash
dayrecord export --target nanobot
```

输出目录默认：`~/.../DayRecord/nanobot-memory/`
