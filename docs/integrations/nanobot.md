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

**前置条件：** 只读工具直接读本地 `dayrecord.db`；触发 AI / 录制控制需要采集服务在运行（GUI 或 `dayrecord daemon`）。

## 工具

### 只读（脱敏，无需采集进程）

| 工具 | 说明 |
|------|------|
| `get_user_profile` | 习惯画像 + 活跃事实 JSON |
| `query_user_facts` | 关键词搜索事实 |
| `get_recent_summary` | 近 N 日复盘 Markdown |
| `get_today_context` | 今日复盘 + 事实 + 任务单元（脱敏 Markdown） |
| `what_working_on_now` | 当前应用/窗口/任务名（脱敏 JSON，无原始键入） |

### 触发 / 控制（需采集服务 + 本地 IPC）

| 工具 | 说明 |
|------|------|
| `generate_today_summary` | 调用 DayRecord 内置 LLM 生成今日复盘 |
| `consolidate_memory` | 行为模式 + 任务单元 + 事实巩固 |
| `pause_recording` / `resume_recording` | 暂停 / 恢复采集 |
| `get_recording_status` | 录制状态 + 今日统计 |

服务未运行时，触发类工具返回结构化错误并提示先启动 GUI 或 `dayrecord daemon`。

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
