# 预编译安装

发布页：[GitHub Releases](https://github.com/mikaku9944/dayrecord/releases)

## CLI（Agent 接入推荐）

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

1. 下载 Release 中的 `.msi` 安装包（名称类似 `DayRecord_<version>_x64-setup.msi`）
2. 安装后从开始菜单启动 DayRecord
3. 若 SmartScreen 提示未知发布者，选择「仍要运行」（未签名构建的正常现象）

## 校验

每个 Release 附带 `SHA256SUMS.txt`，可核对下载文件完整性。

## MCP 配置示例

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

Windows 若未加入 PATH，请写 `dayrecord.exe` 的完整路径。
