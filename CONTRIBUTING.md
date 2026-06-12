# 贡献指南

## 架构约定

- **dayrecord-core**：纯逻辑，禁止 `unsafe`、禁止平台 API
- **dayrecord-adapters**：各平台 Ports 实现
- **dayrecord-runtime**：采集编排（Orchestrator）
- **dayrecord-cli**：CLI + MCP，无 Tauri 依赖
- **dayrecord-app**：Tauri GUI

新增平台能力时：先扩展 `ports.rs` trait，再在 `adapters` 下按 `window_*` / `context_*` / `keyboard_*` 分文件实现。

## 开发

```bash
cargo test -p dayrecord-core -p dayrecord-adapters -p dayrecord-runtime -p dayrecord-cli --lib
cd frontend && npm test
```

Windows GUI：`scripts\run.cmd`

## PR 要求

- `cargo fmt --all`
- 新逻辑附带单元测试（core 优先）
- 跨平台改动需在 CI 三矩阵通过
