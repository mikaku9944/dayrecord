# DayRecord

[![CI](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml/badge.svg)](https://github.com/mikaku9944/dayrecord/actions/workflows/ci.yml)
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

### CLI（推荐 Agent 接入）

```bash
git clone https://github.com/mikaku9944/dayrecord.git
cd dayrecord
cargo install --path crates/dayrecord-cli
```

### 桌面 GUI（Windows 优先）

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

# MCP（给 nanobot 等）
dayrecord mcp

# 文件导出
dayrecord export --target hermes
dayrecord export --target openclaw
dayrecord export --target nanobot
```

数据目录：`dayrecord data-dir`

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

- [隐私说明](PRIVACY.md)
- [贡献指南](CONTRIBUTING.md)
- [打包与分发](docs/packaging.md)
- [macOS 权限](docs/macos-permissions.md)
- [验收清单](docs/acceptance-checklist.md)
- [已知限制](docs/limitations.md)

## 许可证

MIT — 见 [LICENSE](LICENSE)
