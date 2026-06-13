# Install DayRecord CLI from GitHub Release to a stable path for MCP.
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1 -Version 0.1.1 -WriteConfig
param(
    [string]$Version = "",
    [switch]$WriteConfig,
    [string]$Repo = "mikaku9944/dayrecord"
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")

function Get-LatestReleaseVersion {
    param([string]$Repository)
    $api = "https://api.github.com/repos/$Repository/releases/latest"
    $headers = @{ "User-Agent" = "dayrecord-install" }
    $release = Invoke-RestMethod -Uri $api -Headers $headers
    return ($release.tag_name -replace '^v', '')
}

function Get-WindowsArtifact {
    param([string]$Ver)
    return @{
        ArchiveName = "dayrecord-$Ver-x86_64-pc-windows-msvc.zip"
        BinName     = "dayrecord.exe"
    }
}

if (-not $Version) {
    Write-Host "Resolving latest release..."
    $Version = Get-LatestReleaseVersion -Repository $Repo
}
Write-Host "Installing DayRecord $Version for Windows x86_64..."

$artifact = Get-WindowsArtifact -Ver $Version
$installDir = Join-Path $env:LOCALAPPDATA "Programs\dayrecord"
$installExe = Join-Path $installDir $artifact.BinName
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$tmp = Join-Path $env:TEMP "dayrecord-install-$Version"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

$baseUrl = "https://github.com/$Repo/releases/download/v$Version"
$archivePath = Join-Path $tmp $artifact.ArchiveName
$checksumsPath = Join-Path $tmp "SHA256SUMS.txt"

Write-Host "Downloading $($artifact.ArchiveName)..."
Invoke-WebRequest -Uri "$baseUrl/$($artifact.ArchiveName)" -OutFile $archivePath -UseBasicParsing
Invoke-WebRequest -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $checksumsPath -UseBasicParsing

$expectedLine = Get-Content $checksumsPath | Where-Object { $_ -match [regex]::Escape($artifact.ArchiveName) } | Select-Object -First 1
if (-not $expectedLine) {
    Write-Error "SHA256SUMS.txt does not list $($artifact.ArchiveName)"
}
$expectedHash = ($expectedLine -split '\s+', 2)[0].ToLower()
$actualHash = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLower()
if ($expectedHash -ne $actualHash) {
    Write-Error "Checksum mismatch for $($artifact.ArchiveName)"
}
Write-Host "Checksum ok."

Expand-Archive -Path $archivePath -DestinationPath $tmp -Force
$extracted = Get-ChildItem -Path $tmp -Recurse -Filter $artifact.BinName | Select-Object -First 1
if (-not $extracted) {
    Write-Error "Could not find $($artifact.BinName) in archive"
}

if (Test-Path $installExe) {
    Write-Host "Replacing $installExe (close Cursor dayrecord MCP if install fails with access denied)."
}
Copy-Item -Path $extracted.FullName -Destination $installExe -Force
Write-Host "Installed: $installExe"

$cursorConfig = Join-Path $env:USERPROFILE ".cursor\mcp.json"
$codexConfig = Join-Path $env:USERPROFILE ".codex\config.toml"
$exeJson = $installExe -replace '\\', '\\'

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

$codexSnippet = @"
[mcp_servers.dayrecord]
command = '$($installExe -replace '\\', '\\')'
args = ["mcp"]
"@

Write-Host ""
Write-Host "=== Cursor mcp.json snippet ===" -ForegroundColor Cyan
Write-Host $mcpSnippet
Write-Host ""
Write-Host "=== Codex config.toml snippet ===" -ForegroundColor Cyan
Write-Host $codexSnippet

if ($WriteConfig) {
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

    $codexDir = Split-Path $codexConfig -Parent
    if (-not (Test-Path $codexDir)) {
        New-Item -ItemType Directory -Force -Path $codexDir | Out-Null
    }
    if (Test-Path $codexConfig) {
        $body = Get-Content $codexConfig -Raw
        if ($body -notmatch '\[mcp_servers\.dayrecord\]') {
            Add-Content -Path $codexConfig -Value "`n$codexSnippet"
            Write-Host "Appended dayrecord to $codexConfig"
        } else {
            Write-Host "Codex config already has [mcp_servers.dayrecord]; update manually if needed."
        }
    } else {
        $codexSnippet | Set-Content -Path $codexConfig -Encoding UTF8
        Write-Host "Created $codexConfig"
    }
}

Write-Host ""
Write-Host "Next: enable dayrecord in Cursor MCP settings (toggle off then on)." -ForegroundColor Yellow
Write-Host "Running doctor mcp..."
& $installExe doctor mcp
exit $LASTEXITCODE
