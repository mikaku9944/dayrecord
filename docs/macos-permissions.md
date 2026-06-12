# macOS 权限与采集对等

## 构建 feature

```bash
# 可见文本（Accessibility / AX）
cargo build -p dayrecord-app --features dayrecord-adapters/macos-ax

# 键盘（CGEventTap）
cargo build -p dayrecord-app --features dayrecord-adapters/macos-keyboard
```

CLI daemon 同理：`cargo build -p dayrecord-cli --features dayrecord-adapters/macos-ax,dayrecord-adapters/macos-keyboard`

## 系统授权

| 权限 | 路径 | 用途 |
|------|------|------|
| 辅助功能 | 系统设置 → 隐私与安全性 → 辅助功能 | 读取焦点控件可见文本 |
| 输入监控 | 系统设置 → 隐私与安全性 → 输入监控 | 全局键盘事件 |

首次启动 GUI 时，若未授权，托盘菜单会提示前往系统设置开启。

## 与其他平台对比

| 能力 | Windows | macOS | Linux |
|------|---------|-------|-------|
| 窗口 | UIA + 钩子 | NSWorkspace / active-win | active-win |
| 可见文本 | UIA | AX（需 feature + 授权） | AT-SPI（需 feature） |
| 键盘 | WH_KEYBOARD_LL | CGEventTap（需 feature + 授权） | X11 only（Wayland 不支持） |
| 密钥 | Credential Manager | Keychain（keyring） | Secret Service（keyring） |
