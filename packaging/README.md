# Packaging manifests

Optional distribution channels for the DayRecord CLI.

| Channel | Path | Notes |
|---------|------|-------|
| Install script (Windows) | [scripts/install.ps1](../scripts/install.ps1) | Recommended; stable path under `%LOCALAPPDATA%\Programs\dayrecord` |
| Install script (Unix) | [scripts/install.sh](../scripts/install.sh) | `~/.local/bin/dayrecord` |
| Local build install | [scripts/install-local.ps1](../scripts/install-local.ps1) | `cargo install` to stable path |
| npm launcher | [packages/mcp](../packages/mcp) · [@dayrecord/mcp](https://www.npmjs.com/package/@dayrecord/mcp) | `npx -y @dayrecord/mcp` — auto-downloads native CLI on first run |
| Scoop | [scoop/dayrecord.json](scoop/dayrecord.json) | Update hash at release |
| winget | [winget/com.dayrecord.cli.yaml](winget/com.dayrecord.cli.yaml) | Update InstallerSha256 at release |

After install, configure MCP with the installed absolute path or `dayrecord-mcp` binary alias.
