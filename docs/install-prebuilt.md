# 预编译安装

发布页：[GitHub Releases](https://github.com/mikaku9944/dayrecord/releases)

## 一键安装（推荐）

**Windows：**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1 -WriteConfig
```

安装到 `%LOCALAPPDATA%\Programs\dayrecord\dayrecord.exe`，可选写入 `~/.cursor/mcp.json`。

**macOS / Linux：**

```bash
chmod +x scripts/install.sh
./scripts/install.sh --write-config
```

## CLI（手动）

### Windows

1. 下载 `dayrecord-<version>-x86_64-pc-windows-msvc.zip`
2. 解压，将 `dayrecord.exe` 加入 PATH，或放到固定目录
3. 验证：

```powershell
dayrecord --help
dayrecord data-dir
```

### macOS（Apple Silicon）

```bash
tar xzf dayrecord-<version>-aarch64-apple-darwin.tar.gz
sudo mv dayrecord-<version>-aarch64-apple-darwin/dayrecord /usr/local/bin/dayrecord
dayrecord --help
```

### Linux

```bash
tar xzf dayrecord-<version>-x86_64-unknown-linux-gnu.tar.gz
sudo mv dayrecord-<version>-x86_64-unknown-linux-gnu/dayrecord /usr/local/bin/dayrecord
dayrecord --help
```

## Windows 桌面 GUI

从 **v0.1.1** 起提供 `.msi` 安装包（v0.1.0 仅含 CLI）。

1. 下载 Release 中的 `.msi`（名称类似 `DayRecord_<version>_x64-setup.msi`）
2. 安装后从开始菜单启动 DayRecord
3. 若 SmartScreen 提示未知发布者，选择「仍要运行」（未签名构建的正常现象）

## 校验

每个 Release 附带 `SHA256SUMS.txt`，可核对下载文件完整性。

## MCP 配置示例

**推荐 — npx（[已发布 npm](https://www.npmjs.com/package/@dayrecord/mcp)）**：只需 Node.js 18+，首次运行自动下载原生 CLI。

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

macOS / Linux 将 `command` 改为 `npx`，`args` 为 `["-y", "@dayrecord/mcp"]`。

**原生二进制（Windows）** — `install.ps1` 装到 `%LOCALAPPDATA%\Programs\dayrecord\dayrecord.exe`；`install-local.ps1` / npx 自动下载装到 `...\dayrecord\bin\`：

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "C:\\Users\\YOU\\AppData\\Local\\Programs\\dayrecord\\dayrecord.exe",
      "args": ["mcp"]
    }
  }
}
```

（若二进制在 `bin\` 子目录，路径中加上 `bin\\`。）
