# 打包与分发

## CLI / MCP 二进制

```bash
cargo install --path crates/dayrecord-cli
```

GitHub Releases 建议附带三平台预编译包：

| 平台 | 产物 |
|------|------|
| Windows | `dayrecord-x86_64-pc-windows-msvc.zip` |
| macOS | `dayrecord-aarch64-apple-darwin.tar.gz` |
| Linux | `dayrecord-x86_64-unknown-linux-gnu.tar.gz` |

### 包管理器（社区维护）

- **Homebrew**（macOS/Linux）：`brew install dayrecord`（需提交 formula 到 homebrew-core 或 tap）
- **Scoop**（Windows）：`scoop install dayrecord`（需 manifest JSON）
- **AUR**（Arch）：`yay -S dayrecord`（需 PKGBUILD）

## Tauri GUI

```bash
cd frontend && npm run build && cd ..
cargo tauri build -p dayrecord-app
```

| 平台 | 产物 | 签名说明 |
|------|------|----------|
| Windows | `.msi` / `.exe` | 代码签名证书可降低 SmartScreen 拦截；未签名需用户确认 |
| macOS | `.dmg` / `.app` | Apple Developer ID + `notarize`；首次需用户在「隐私与安全性」允许 |
| Linux | `.deb` / `.AppImage` | 一般无需签名；AppImage 需 `libwebkit2gtk` 等依赖 |

### macOS 权限

GUI 与 CLI daemon 采集前需用户在「系统设置 → 隐私与安全性」授予：

- **辅助功能** — AX 可见文本（`--features macos-ax`）
- **输入监控** — 键盘采集（`--features macos-keyboard`）

详见 [macOS 权限说明](macos-permissions.md)。

## CI

`.github/workflows/ci.yml` 在三平台矩阵运行 `cargo test`、`cargo clippy`、前端 vitest；Windows 额外 `cargo build -p dayrecord-app`。
