# DayRecord 验收可追溯矩阵

PRD §12 验收项与自动测试 / 手动清单映射。

| PRD §12 | 验收项 | 验证方式 | 测试 / 文档 |
|---------|--------|----------|-------------|
| 1 | 首启隐私同意 | 手动 | [acceptance-checklist.md](./acceptance-checklist.md) #1；`frontend/src/main.ts` 同意弹窗 |
| 2 | 英文输入与粘贴 | 手动 + 单测 | 清单 #2；`dayrecord-core` session/ime 单测 |
| 3 | 会话落盘（30s/切窗） | 手动 + 单测 | 清单 #3；`domain::session` 边界测试 |
| 4 | 前台窗口时间轴 | 手动 + 单测 | 清单 #4；`domain::activity` 单测 |
| 5 | 空闲 >60s 不计 | 手动 + 单测 | 清单 #5；`is_idle_gap` rstest |
| 6 | 暂停录制 | 手动 + 单测 | 清单 #6；`orchestrator::recording_switch_blocks_events` |
| 7 | 生成复盘 | 手动 + 集测 | 清单 #7；`prompt`/`summary` 单测；`llm` wiremock；`orchestrator::generates_summary` |
| 8 | 托盘操作 | 手动 | 清单 #8；`dayrecord-app/src/lib.rs` tray 接线 |
| 9 | 清空数据 | 手动 + 集测 | 清单 #9；`repository::clear_preserves_consent` |

## M8 记忆进化层

| 项 | 验证方式 | 测试 / 文档 |
|----|----------|-------------|
| facts 表 + FTS5 | 集测 | `repository::facts_fts_search` |
| consolidation 纯逻辑 | 单测 | `dayrecord-core::consolidation` |
| facts UI 查看/删除 | 手动 | 清单 #10–11；`frontend` facts 列表 |
| 缺口记录 | 文档 | [memory-gap-log.md](./memory-gap-log.md) |

## 可选 / 本地

| 项 | 命令 |
|----|------|
| DeepSeek live smoke | `DEEPSEEK_API_KEY=... cargo test -p dayrecord-adapters --test live_deepseek -- --ignored` |
| 键盘 SendInput 集测 | `cargo test -p dayrecord-adapters --test keyboard_sendinput -- --ignored` |
