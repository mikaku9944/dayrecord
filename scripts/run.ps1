# Build frontend assets and run DayRecord (no Vite dev server required).
# In CMD, use: scripts\run.cmd  (do not run this .ps1 file directly in CMD)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")

$env:Path = "$env:USERPROFILE\.cargo\bin;C:\hqproject\nodejs;" + $env:Path

# Optional per-machine overrides (gitignored): copy scripts/local.env.ps1.example
$localEnv = Join-Path $PSScriptRoot "local.env.ps1"
if (Test-Path $localEnv) { . $localEnv }

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Error "找不到 npm。请先安装 Node.js 并将其加入 PATH。"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "找不到 cargo。请先安装 Rust (rustup) 并重启终端。"
}

Push-Location (Join-Path $root "frontend")
npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Pop-Location

Remove-Item Env:TAURI_CONFIG -ErrorAction SilentlyContinue

Push-Location $root
cargo run -p dayrecord-app
exit $LASTEXITCODE
