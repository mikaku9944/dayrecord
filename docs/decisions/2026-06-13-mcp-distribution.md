# 2026-06-13 — MCP 分发与接入决议

## 背景

DayRecord 需支持 **Cursor / Codex / nanobot** 等 Agent 通过 MCP 读取用户上下文，并在用户同意采集后触发复盘与控制。当日完成从「能跑 MCP」到「可一键安装、可 npm 分发」的闭环。

## 里程碑与提交映射

| 里程碑 | 主题 | 预期 commit 前缀 |
|--------|------|-------------------|
| **M0** | 决议文档（本文件） | `docs(decisions):` |
| **M1** | 控制协议扩展（自启拒绝原因） | `feat(core):` |
| **M2** | 运行时按需拉起 daemon | `feat(runtime):` |
| **M3** | MCP 工具契约与 Schema 兼容失败 | `feat(mcp):` |
| **M4** | CLI 库化、doctor、`dayrecord-mcp` 入口 | `refactor(cli):` |
| **M5** | 安装脚本与稳定路径 | `feat(install):` |
| **M6** | npm 与包管理器清单 | `feat(distribution):` |
| **M7** | 用户文档与快速接入 | `docs:` |

---

## M1 — 控制协议

### D1. 区分「数据库录制开关」与「IPC 在线」

- **决议**：MCP 状态类输出同时暴露 `recording_state_db`（持久化设置）与 `control_ipc_online`（采集 IPC 是否可达）；`get_recording_status` 在 IPC 离线时仍返回 **成功** 结构，不触发自启。
- **理由**：Agent 与 GUI 可能不同步；只读查询不应隐式启动采集服务。
- **实现**：`RecordingStatusOutput`；`get_recording_status` 使用只读 `IpcControlClient`。

### D2. 自启拒绝为显式错误类型

- **决议**：新增 `ControlError::AutostartDenied`，用于 consent 未授予或 `mcp_autostart_daemon=false`。
- **理由**：与「服务未运行」区分，便于 MCP 返回可操作的 `hint`。

---

## M2 — Daemon 按需自启

### D3. MCP 不常驻 daemon；控制类工具可触发 detached 自启

- **决议**：
  - `npx @dayrecord/mcp` / `dayrecord mcp` **仅**启动 MCP stdio 进程。
  - `dayrecord daemon`（或 GUI）为独立采集 + IPC 进程。
  - 控制类工具（pause / resume / generate / consolidate）经 `AutoStartControlClient`：IPC 离线且策略允许时，detached 执行 `dayrecord daemon`，轮询最多 5s。
- **理由**：隐私默认最小化；只读工具无需后台采集；符合「先 consent 再采集」。

### D4. 自启三门禁（须同时满足）

1. `consent=true`（CLI 或 GUI）
2. `mcp_autostart_daemon` 不为 `false`（默认允许）
3. 未设置 `DAYRECORD_MCP_DISABLE_AUTOSTART`（doctor 探针用）

### D5. Daemon 尊重持久化 `recording` 设置

- **决议**：`dayrecord daemon` 启动时读取 DB 中 `recording`，不再强制 `true`。
- **理由**：用户暂停后重启 daemon 不应被悄悄恢复录制。

---

## M3 — MCP 工具契约

### D6. 业务失败使用 `isError` + 结构化内容（早期）

- **决议**：控制类业务失败经 `ToolFail` → `CallToolResult::structured_error`，设置 `isError: true`。
- **理由**：与 MCP 规范及 Cursor 客户端语义一致。

### D7. 控制类失败须符合声明的 output Schema（当日增补）

- **决议**：IPC 离线、无数据等业务失败改为 **`ok: false` 的成功响应体**（`ControlAck` / `MarkdownOutput` / `ConsolidateOutput` 内嵌 `error` / `hint`），而非仅返回 `McpErrorJson` 形态。
- **理由**：Cursor 等对 `structuredContent` 做严格 output Schema 校验；`isError` 与 Schema 形状不一致会导致 Agent 报「Schema 不匹配」。
- **非目标**：只读工具仍可对真实异常使用 `ToolFail`。

### D8. 工具注解与 serverInfo

- **决议**：只读工具 `read_only_hint = true`；副作用工具 `read_only_hint = false`；`serverInfo.name = "dayrecord"`，版本来自 crate。
- **理由**：客户端可安全并行调度只读工具。

### D9. Workspace `schemars` 统一为 1.x

- **决议**：与 `rmcp` 1.0 对齐，避免 0.8 / 1.0 混用导致工具 Schema 生成失败。

---

## M4 — CLI 形态

### D10. `dayrecord-cli` 提供 lib + 双 bin

- **决议**：
  - `dayrecord` — 完整 CLI
  - `dayrecord-mcp` — 仅 MCP stdio（无 `mcp` 子命令）
  - 公共逻辑在 `dayrecord_cli` lib
- **理由**：`mcp.json` 可配置更短命令行；测试可复用 handlers。

### D11. `dayrecord doctor mcp` 健康检查

- **决议**：探测二进制、`tools/list` 数量、IPC 离线时 pause 的失败信号、consent 允许时的 daemon 自启 + 清理。
- **理由**：安装脚本与用户自助排障。

---

## M5 — 安装与稳定路径

### D12. 三层安装路径

| 方式 | 典型路径 (Windows) | 受众 |
|------|-------------------|------|
| `install.ps1` / `install.sh` | `%LOCALAPPDATA%\Programs\dayrecord\dayrecord.exe` | Release 用户 |
| `install-local.ps1` | `...\Programs\dayrecord\bin\`（`cargo install --root`） | 开发者 |
| `npx @dayrecord/mcp` 首次下载 | `...\Programs\dayrecord\bin\` | Cursor 一键 |

- **决议**：文档明确路径差异；`DAYRECORD_BIN` 可覆盖。
- **已知**：`install.ps1` 与 `bin\` 布局尚未完全统一（后续可收敛）。

### D13. `run.cmd` 仅 GUI

- **决议**：不通过 `run.cmd` 更新 MCP/CLI 二进制；MCP 由 IDE 按 `mcp.json` 独立拉起。

---

## M6 — 分发渠道

### D14. `@dayrecord/mcp` 为薄 npm 启动器

- **决议**：
  - npm 包 **不包含** Rust 源码；仅 Node 脚本 + 首次从 GitHub Release 下载原生二进制（SHA256 校验）。
  - 需要 npm 组织 `@dayrecord`；`publishConfig.access = public`。
- **理由**：`npx -y @dayrecord/mcp` 对 Cursor 用户零 Rust/Git 依赖；与 memory-server 类体验一致。

### D15. Scoop / winget 清单为模板

- **决议**：`packaging/scoop`、`packaging/winget` 随 Release 更新 hash；非阻塞主路径。

---

## M7 — 用户可见行为摘要

| 场景 | 预期 |
|------|------|
| 仅配置 npx MCP | 只读工具立即可用 |
| 未 `consent` | 控制类返回 `ok: false` + hint；daemon 不自启 |
| 已 `consent`，调控制工具 | MCP 尝试自启 daemon |
| 刚安装无活动数据 | `what_working_on_now` 提示无数据；generate/consolidate 业务失败但 Schema 合法 |
| 需要常驻采集 | 用户开 GUI 或 `dayrecord daemon` |

---

## 暂缓 / 后续

- CI：Release tag 自动 `npm publish`
- 统一 `install.ps1` 安装到 `bin\` 子目录
- MCP 连接时可选「consent 后预热 daemon」（产品决策，当前不做）
- 代码签名以降低 Windows SmartScreen 摩擦

---

## 参考

- [README MCP 快速接入](../../README.md#mcp-快速接入)
- [packages/mcp/README.md](../../packages/mcp/README.md)
- [install-prebuilt.md](../install-prebuilt.md)
- [packaging.md](../packaging.md)
