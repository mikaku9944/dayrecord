# @dayrecord/mcp

[![npm](https://img.shields.io/npm/v/@dayrecord/mcp)](https://www.npmjs.com/package/@dayrecord/mcp)

npx-friendly MCP launcher for [DayRecord](https://github.com/mikaku9944/dayrecord). On first run it downloads the latest native CLI from GitHub Releases, then starts `dayrecord mcp` on stdio.

**Requires:** Node.js 18+ (for `npx`). No Rust toolchain or git clone needed.

## Cursor `mcp.json` (Windows)

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

## macOS / Linux

```json
{
  "mcpServers": {
    "dayrecord": {
      "command": "npx",
      "args": ["-y", "@dayrecord/mcp"]
    }
  }
}
```

Then enable **dayrecord** in Cursor MCP settings (toggle off → on).

## First run

1. `npx` fetches this package.
2. Launcher downloads `dayrecord` / `dayrecord.exe` to:
   - Windows: `%LOCALAPPDATA%\Programs\dayrecord\bin\`
   - Unix: `~/.local/share/dayrecord/bin/`
3. MCP stdio server starts.

## Control tools

Read-only tools work immediately. For trigger/control tools (generate summary, pause recording), grant consent once:

```bash
dayrecord consent --accept true
```

(Use the downloaded binary path, or add that `bin` folder to PATH.)

## Environment

| Variable | Purpose |
|----------|---------|
| `DAYRECORD_BIN` | Skip search/download; use this executable |
| `DAYRECORD_VERSION` | Pin release version (e.g. `0.1.1`) |
| `DAYRECORD_GITHUB_REPO` | Override repo (default `mikaku9944/dayrecord`) |

## Native install (alternative)

See the main repo [install scripts](https://github.com/mikaku9944/dayrecord/blob/main/docs/install-prebuilt.md) if you prefer not to use npm.
