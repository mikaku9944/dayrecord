# Build and install DayRecord CLI from local source to stable path (developers).
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-local.ps1
param(
    [switch]$WriteConfig
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$installRoot = Join-Path $env:LOCALAPPDATA "Programs\dayrecord"
$installBin = Join-Path $installRoot "bin"
$installExe = Join-Path $installBin "dayrecord.exe"
$installMcp = Join-Path $installBin "dayrecord-mcp.exe"

Write-Host "=== DayRecord local install ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Before continuing:" -ForegroundColor Yellow
Write-Host "  1. Turn OFF dayrecord MCP in Cursor (avoids 'access denied' on exe replace)."
Write-Host "  2. Stop any running dayrecord.exe MCP/daemon processes if upgrade fails."
Write-Host ""

$procs = Get-Process dayrecord -ErrorAction SilentlyContinue
if ($procs) {
    Write-Host "Running dayrecord processes:" -ForegroundColor Yellow
    $procs | Format-Table Id, Path -AutoSize
    $answer = Read-Host "Kill them now? [y/N]"
    if ($answer -match '^[yY]') {
        Stop-Process -Name dayrecord -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
}

New-Item -ItemType Directory -Force -Path $installBin | Out-Null
Push-Location $root
try {
    cargo install --path crates/dayrecord-cli --root $installRoot --force
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Installed: $installExe" -ForegroundColor Green
if (Test-Path $installMcp) {
    Write-Host "Installed: $installMcp (MCP alias — use with empty args in mcp.json)" -ForegroundColor Green
}

$mcpSnippet = @"
{
  "mcpServers": {
    "dayrecord": {
      "command": "$installExe",
      "args": ["mcp"]
    }
  }
}
"@

Write-Host ""
Write-Host "=== Cursor mcp.json snippet ===" -ForegroundColor Cyan
Write-Host $mcpSnippet

if ($WriteConfig) {
    $cursorConfig = Join-Path $env:USERPROFILE ".cursor\mcp.json"
    $cursorDir = Split-Path $cursorConfig -Parent
    if (-not (Test-Path $cursorDir)) {
        New-Item -ItemType Directory -Force -Path $cursorDir | Out-Null
    }
    if (Test-Path $cursorConfig) {
        $existing = Get-Content $cursorConfig -Raw | ConvertFrom-Json
        if (-not $existing.mcpServers) {
            $existing | Add-Member -NotePropertyName mcpServers -NotePropertyValue (@{})
        }
        $existing.mcpServers.dayrecord = @{
            command = $installExe
            args    = @("mcp")
        }
        ($existing | ConvertTo-Json -Depth 10) | Set-Content -Path $cursorConfig -Encoding UTF8
        Write-Host "Updated $cursorConfig"
    } else {
        $mcpSnippet | Set-Content -Path $cursorConfig -Encoding UTF8
        Write-Host "Created $cursorConfig"
    }
}

Write-Host ""
Write-Host "Note: scripts\run.cmd starts the GUI (dayrecord-app) only; it does NOT update this CLI." -ForegroundColor Yellow
Write-Host "Re-enable dayrecord MCP in Cursor, then run doctor:"
& $installExe doctor mcp
exit $LASTEXITCODE
