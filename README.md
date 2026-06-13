# DayRecord

[![CI](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml/badge.svg)](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml)
[![Release](https://github.com/mikaku9944/dayrecord/actions/workflows/release.yml/badge.svg)](https://github.com/mikaku9944/dayrecord/releases)
[![npm @dayrecord/mcp](https://img.shields.io/npm/v/@dayrecord/mcp)](https://www.npmjs.com/package/@dayrecord/mcp)
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

适用于 **Cursor**、Claude Desktop、Codex、nanobot 等。配置好后重载 MCP 即可使用；**只读工具**直接读本地 `dayrecord.db`；**控制类工具**在已同意采集（`consent=true`）时会按需自动拉起后台 `dayrecord daemon`（可用设置 `mcp_autostart_daemon=false` 关闭）。

### 1. Cursor 一键（推荐，已发布 npm）

只需 Node.js 18+，**无需** clone 仓库或先跑安装脚本。`npx` 会拉取 [@dayrecord/mcp](https://www.npmjs.com/package/@dayrecord/mcp)，首次运行自动下载 GitHub Release 原生 CLI。

**Windows：**

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "cmd",
      "args": ["/c", "npx", "-y", "@dayrecord/mcp"]
    }
  }
}
```

**macOS / Linux：** `"command": "npx"`, `"args": ["-y", "@dayrecord/mcp"]`

详见 [packages/mcp/README.md](packages/mcp/README.md)。

### 2. 原生二进制安装（可选）

**Windows（`install.ps1` → `%LOCALAPPDATA%\Programs\dayrecord\dayrecord.exe`）：**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1 -WriteConfig
```

**macOS / Linux（`~/.local/bin/dayrecord`）：**

```bash
chmod +x scripts/install.sh
./scripts/install.sh --write-config
```

**从源码构建到稳定路径（开发者，`cargo install` → `...\dayrecord\bin\`）：**

```powershell
powershell -File scripts\install-local.ps1 -WriteConfig
```

> `scripts\run.cmd` 只启动 **GUI**（`dayrecord-app`），不会更新 CLI/MCP 二进制。MCP 由 IDE 按 `mcp.json` 独立拉起。

**`mcp.json` 示例（原生路径，无 npm）：**

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "C:\\Users\\YOU\\AppData\\Local\\Programs\\dayrecord\\bin\\dayrecord.exe",
      "args": ["mcp"]
    }
  }
}
```

（`install.ps1` 装到上级目录时把路径中的 `bin\\` 去掉即可。）或使用 MCP 专用入口：

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "C:\\Users\\YOU\\AppData\\Local\\Programs\\dayrecord\\bin\\dayrecord-mcp.exe",
      "args": []
    }
  }
}
```

### 3. 启用并验证

1. 在 MCP 设置中**开启** `dayrecord`（改配置后建议关 → 开）。
2. 首次使用控制类工具前：`dayrecord consent --accept true`（或在 GUI 中同意采集）。
3. 正常应看到 **10 个 tools** 与 **3 个 resources**。
4. 验证：

```bash
dayrecord --version
dayrecord mcp --version
dayrecord doctor mcp
```

`doctor mcp` 检查二进制漂移、`tools/list`、控制类 `isError`、以及 daemon 自启 + IPC（在 consent 允许时）。

### Codex（`~/.codex/config.toml`）

```toml
[mcp_servers.dayrecord]
command = 'C:\\Users\\YOU\\AppData\\Local\\Programs\\dayrecord\\bin\\dayrecord.exe'
args = ["mcp"]
```

或使用 npx：`command = "npx"`, `args = ["-y", "@dayrecord/mcp"]`

### 4. 工具一览

| 类型 | 工具 | 需要采集服务？ |
|------|------|----------------|
| 只读 | `get_user_profile` · `query_user_facts` · `get_recent_summary` · `get_today_context` · `what_working_on_now` · `get_recording_status` | 否（读 DB；状态查询不触发自启） |
| 触发 / 控制 | `generate_today_summary` · `consolidate_memory` · `pause_recording` · `resume_recording` | 是（MCP 可自动拉起 daemon，需 `consent=true`） |

触发类在 `consent` 未同意或 `mcp_autostart_daemon=false` 时返回 `isError: true` 及结构化 hint。

**Resources：** `dayrecord://user/profile` · `dayrecord://facts/active` · `dayrecord://context/today` · `dayrecord://memory/{YYYY-MM-DD}`

API Key 在 GUI 设置中配置（keyring）；未配置时触发类走 Mock LLM（仅适合开发）。

更多细节：[nanobot 接入](docs/integrations/nanobot.md) · [Agent 上下文说明](docs/AGENT-CONTEXT.md) · [预编译安装](docs/install-prebuilt.md) · [Scoop/winget 清单](packaging/)

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
