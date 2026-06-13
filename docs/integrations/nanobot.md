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

安装方式（任选其一，**Cursor 推荐 npx**）：

```json
// 仅需 Node.js 18+，无需预先安装原生 CLI
// mcp.json → "command": "cmd", "args": ["/c", "npx", "-y", "@dayrecord/mcp"]
```

```powershell
# Windows 一键（Release 二进制）
powershell -File scripts\install.ps1 -WriteConfig
```

或 `scripts\install-local.ps1`（从源码构建）。npm 包：[@dayrecord/mcp](https://www.npmjs.com/package/@dayrecord/mcp)。

**前置条件：** 只读工具直接读 `dayrecord.db`；控制类工具在 `consent=true` 时 MCP 可自动拉起 `dayrecord daemon`（`mcp_autostart_daemon=false` 可关闭）。

验证安装与 MCP 健康：

```bash
dayrecord --version
dayrecord mcp --version
dayrecord doctor mcp
```

`get_recording_status` 返回 `recording_state_db`（数据库设置）与 `control_ipc_online`（采集 IPC 是否在线），二者可能不一致。

## Codex

在 `~/.codex/config.toml`：

```toml
[mcp_servers.dayrecord]
command = "dayrecord"
args = ["mcp"]
```

## 工具

### 只读（脱敏，无需采集进程）

工具描述带 `[Read-only]`，`read_only_hint = true`。

| 工具 | 说明 |
|------|------|
| `get_user_profile` | 习惯画像 + 活跃事实（结构化 JSON 对象） |
| `query_user_facts` | 关键词搜索事实 |
| `get_recent_summary` | 近 N 日复盘 Markdown（`{ markdown }`） |
| `get_today_context` | 今日复盘 + 事实 + 任务单元（`{ markdown }`） |
| `what_working_on_now` | 当前应用/窗口/任务名（结构化 JSON） |
| `get_recording_status` | `recording_state_db` + `control_ipc_online`（IPC 离线也返回成功） |

### 触发 / 控制（需采集服务 + 本地 IPC）

工具描述带 `[Side-effect]`。失败时 MCP `isError: true`，并附带 `recording_state_db` / `control_ipc_online`。

| 工具 | 说明 |
|------|------|
| `generate_today_summary` | 调用 DayRecord 内置 LLM 生成今日复盘 |
| `consolidate_memory` | 行为模式 + 任务单元 + 事实巩固 |
| `pause_recording` / `resume_recording` | 暂停 / 恢复采集 |

服务未运行且未同意采集时，触发类工具返回 `isError: true` 及结构化 hint；同意采集后 MCP 可自动拉起 daemon。

## 资源（MCP resources）

| URI | 说明 |
|-----|------|
| `dayrecord://user/profile` | 用户画像 + 活跃事实 JSON |
| `dayrecord://facts/active` | 全部活跃事实 JSON |
| `dayrecord://context/today` | 今日脱敏上下文 Markdown |
| `dayrecord://memory/{date}` | 指定日期复盘 Markdown（如 `2026-06-10`） |

## LLM 配置

- API Key：GUI 设置或 keyring `deepseek_api_key`
- 可选 settings：`llm_base_url`（本地 OpenAI 兼容端点）、`llm_model`（模型名）
- 未配置 Key 时触发类工具使用 Mock LLM（仅开发/测试）

## 文件兜底

```bash
dayrecord export --target nanobot
```

输出目录默认：`~/.../DayRecord/nanobot-memory/`
