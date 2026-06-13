# DayRecord

[![CI](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml/badge.svg)](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml)
[![Release](https://github.com/mikaku9944/dayrecord/actions/workflows/release.yml/badge.svg)](https://github.com/mikaku9944/dayrecord/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**看得懂你在干什么，却从不截一张图。**

跨平台个人工作记忆助手：读取界面可见文本（无截图）、键盘输入与窗口时间轴，生成工作复盘，并通过 **CLI / MCP / 文件导出** 为 Hermes、nanobot、OpenClaw（小龙虾）等 Agent 提供用户上下文。

## 主打能力

| 能力 | 说明 |
|------|------|
| **无截图读上下文** | Windows UIA / macOS AX / Linux AT-SPI（可选） |
| **按需 AI** | 仅生成复盘或抽取事实时调用 DeepSeek |
| **Agent 接入** | `dayrecord context` · `dayrecord mcp` · `dayrecord export` |
| **六边形架构** | core / adapters / runtime / cli / app |

## 架构

| Crate | 职责 |
|-------|------|
| `dayrecord-core` | 领域逻辑、ContextBundle、connect 导出、paths |
| `dayrecord-adapters` | SQLite、DeepSeek、平台采集、keyring |
| `dayrecord-runtime` | Orchestrator 编排 |
| `dayrecord-cli` | CLI + MCP server（单二进制） |
| `dayrecord-app` | Tauri 2 GUI + 托盘 |
| `frontend/` | TypeScript UI |

## 安装

### 预编译（开箱即用，推荐）

从 [GitHub Releases](https://github.com/mikaku9944/dayrecord/releases) 下载：

| 平台 | 产物 |
|------|------|
| Windows | `dayrecord-*.zip`（CLI）+ `.msi`（GUI） |
| macOS (ARM) | `dayrecord-*-aarch64-apple-darwin.tar.gz` |
| Linux x64 | `dayrecord-*-x86_64-unknown-linux-gnu.tar.gz` |

详细步骤见 [预编译安装说明](docs/install-prebuilt.md)。

### 从源码构建

**CLI：**

```bash
git clone https://github.com/mikaku9944/dayrecord.git
cd dayrecord
cargo install --path crates/dayrecord-cli
```

**桌面 GUI（Windows 优先）：**

```powershell
git clone https://github.com/mikaku9944/dayrecord.git
cd dayrecord
cd frontend && npm install && cd ..
scripts\run.cmd
```

热更新开发：`scripts\dev.cmd`

## 快速使用（Agent）

```bash
# JSON 用户画像 + 事实
dayrecord context --scope user --format json

# 近 7 日复盘
dayrecord context --scope recent:7 --format md

# MCP（给 nanobot 等；需先启动 GUI 或 dayrecord daemon 才能触发 AI/控制录制）
dayrecord mcp

# 文件导出
dayrecord export --target hermes
dayrecord export --target openclaw
dayrecord export --target nanobot
```

数据目录：`dayrecord data-dir`

## MCP 快速接入

适用于 **Cursor**、Claude Desktop、nanobot 等支持 MCP 的 Agent。DayRecord MCP 通过 stdio 启动，**不暴露原始键入**，只返回脱敏画像、复盘与行为洞察。

### 1. 安装 CLI

```bash
# 预编译：见 docs/install-prebuilt.md
# 或从源码
cargo install --path crates/dayrecord-cli
dayrecord data-dir   # 确认能运行；数据库路径因平台而异
```

Windows 若未加入 PATH，在配置里写 `dayrecord.exe` 的**完整路径**（例如 `C:\\Users\\YOU\\.cargo\\bin\\dayrecord.exe`）。

### 2. 写入 MCP 配置

**Cursor**（`%USERPROFILE%\.cursor\mcp.json` 或 `~/.cursor/mcp.json`）：

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

`command` 不在 PATH 时改为绝对路径。其他客户端的配置格式相同，仅文件路径不同（见各产品文档）。

### 3. 启用并验证

1. 在 MCP 设置中**开启** `dayrecord`（改配置后建议关 → 开，或重启 IDE）。
2. 正常应看到 **10 个 tools** 与 **3 个 resources**（另有模板 `dayrecord://memory/{date}`）。
3. 让 Agent 试调只读工具，例如 `get_today_context` 或读取资源 `dayrecord://context/today`。

### 4. 工具一览

| 类型 | 工具 | 需要采集服务？ |
|------|------|----------------|
| 只读 | `get_user_profile` · `query_user_facts` · `get_recent_summary` · `get_today_context` · `what_working_on_now` | 否（读本地 `dayrecord.db`） |
| 触发 / 控制 | `generate_today_summary` · `consolidate_memory` · `pause_recording` · `resume_recording` · `get_recording_status` | 是（GUI 或 `dayrecord daemon`） |

**Resources：** `dayrecord://user/profile` · `dayrecord://facts/active` · `dayrecord://context/today` · `dayrecord://memory/{YYYY-MM-DD}`

触发类工具在采集服务未运行时会返回结构化错误，提示先启动 GUI 或 daemon。API Key 在 GUI 设置中配置（keyring）；未配置时触发类走 Mock LLM（仅适合开发）。

更多细节（LLM 配置、文件导出兜底）：[nanobot 接入](docs/integrations/nanobot.md) · [Agent 上下文说明](docs/AGENT-CONTEXT.md)

## 平台支持

| 平台 | 窗口 | 可见文本 | 键盘 | 密钥 |
|------|------|----------|------|------|
| Windows | 完整 | UIA | WH_KEYBOARD_LL | keyring |
| macOS | 完整 | AX（feature） | CGEventTap（feature） | keyring |
| Linux | 完整 | AT-SPI（feature） | 仅 X11（feature）；Wayland 不支持 | keyring |

## Agent 接入文档

- [Hermes](docs/integrations/hermes.md)
- [nanobot](docs/integrations/nanobot.md)
- [OpenClaw / 小龙虾](docs/integrations/openclaw.md)

## 开发

```bash
cargo test -p dayrecord-core -p dayrecord-adapters -p dayrecord-runtime -p dayrecord-cli --lib
cd frontend && npm test
```

## 文档

- [Agent 上下文说明](docs/AGENT-CONTEXT.md)
- [隐私说明](PRIVACY.md)
- [贡献指南](CONTRIBUTING.md)
- [预编译安装](docs/install-prebuilt.md)
- [打包与分发](docs/packaging.md)
- [macOS 权限](docs/macos-permissions.md)
- [验收清单](docs/acceptance-checklist.md)
- [已知限制](docs/limitations.md)

## 许可证

MIT — 见 [LICENSE](LICENSE)
