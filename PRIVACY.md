# DayRecord 隐私说明

## 原则

- **本地优先**：原始键盘记录、窗口标题、UIA 文本默认仅存本机 SQLite。
- **按需上云**：仅在你点击「生成复盘」或「抽取事实」时，将**脱敏后的摘要**发送至 DeepSeek API。
- **Agent 导出**：CLI / MCP / 文件导出仅包含习惯画像、抽取事实与复盘，**不含原始 keystroke 流**。

## 数据存储

| 数据 | 位置 | 说明 |
|------|------|------|
| 活动与会话 | `dayrecord.db` | 各平台数据目录见 `dayrecord data-dir` |
| API Key | 系统密钥链 (keyring) | Windows Credential / macOS Keychain / Linux Secret Service |
| Agent 导出 | `agent-export/` 等 | 用户可删除或自定义路径 |

## 权限

| 平台 | 权限 | 用途 |
|------|------|------|
| Windows | 无额外弹窗（钩子可能触发杀软提示） | 键盘、窗口、UIA |
| macOS | 辅助功能、输入监控 | AX 文本、键盘（需 feature 构建） |
| Linux | AT-SPI（可选） | 可见文本；Wayland 不支持全局键盘钩子 |

## 你的控制

- 首次启动需明确同意采集
- 可随时暂停录制、清空数据
- 可仅使用 `dayrecord context` 只读导出，不运行 GUI
